// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use async_trait::async_trait;
use chronicle::{
    db::{model::stardust::milestone::MilestoneRecord, MongoDb},
    runtime::{Actor, ActorContext, ActorError, Addr, ConfigureActor, HandleEvent, Report, RuntimeError},
};
use inx::{client::InxClient, proto::NoParams, tonic::Channel, NodeStatus};
use serde::{Deserialize, Serialize};

use crate::collector::{
    solidifier::{Solidifier, SolidifierError},
    stardust_inx::MilestoneState,
};

#[derive(Debug, thiserror::Error)]
pub enum SyncerError {
    #[error("INX type conversion error: {0:?}")]
    InxTypeConversion(inx::Error),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error("request error: {0}")]
    Request(#[from] inx::tonic::Status),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncKind {
    #[serde(rename = "max_milestones")]
    Max(u32),
    #[serde(rename = "from_milestone")]
    From(u32),
}

impl Default for SyncKind {
    fn default() -> Self {
        Self::From(1)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncerConfig {
    pub sync_kind: SyncKind,
    pub max_parallel_requests: usize,
    pub solidifier_count: usize,
}

impl SyncerConfig {
    const MAX_SOLIDIFIERS: usize = 100;

    pub fn set_solidifier_count(&mut self, solidifier_count: usize) {
        self.solidifier_count = solidifier_count.clamp(1, Self::MAX_SOLIDIFIERS);
    }
}

impl Default for SyncerConfig {
    fn default() -> Self {
        Self {
            sync_kind: Default::default(),
            max_parallel_requests: 10,
            solidifier_count: 10,
        }
    }
}

// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub struct Syncer {
    db: MongoDb,
    config: SyncerConfig,
    inx_client: InxClient<Channel>,
    latest_ms: u32,
}

impl Syncer {
    pub fn new(db: MongoDb, config: SyncerConfig, inx_client: InxClient<Channel>, latest_ms: u32) -> Self {
        Self {
            db,
            config,
            inx_client,
            latest_ms,
        }
    }

    async fn is_synced(&self, index: u32) -> Result<bool, SyncerError> {
        Ok(self.db.get_sync_record_by_index(index).await?.is_some())
    }
}

#[derive(Debug, Default)]
pub struct SyncerState {
    gaps: Gaps,
    solidifiers: Box<[Addr<Solidifier>]>,
}

#[derive(Debug, Default)]
pub struct Gaps(Vec<Range<u32>>);

impl Iterator for Gaps {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(range) = self.0.first() {
            if range.start >= range.end {
                self.0.remove(0);
            } else {
                break;
            }
        }
        if let Some(range) = self.0.first_mut() {
            let next = range.start;
            range.start += 1;
            Some(next)
        } else {
            None
        }
    }
}

#[async_trait]
impl Actor for Syncer {
    type State = SyncerState;
    type Error = SyncerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        // Request the node status so we can get the pruning index
        let node_status = NodeStatus::try_from(self.inx_client.read_node_status(NoParams {}).await?.into_inner())
            .map_err(SyncerError::InxTypeConversion)?;
        let configured_start = match self.config.sync_kind {
            SyncKind::Max(ms) => self.latest_ms - ms,
            SyncKind::From(ms) => ms,
        };
        let sync_data = self
            .db
            .get_sync_data(configured_start.max(node_status.pruning_index), self.latest_ms)
            .await?
            .gaps;
        if !sync_data.is_empty() {
            let mut solidifiers = Vec::new();
            for i in 0..self.config.solidifier_count {
                solidifiers.push(
                    cx.spawn_child(Solidifier::new(i + 1, self.db.clone()).with_registration(false))
                        .await,
                );
            }
            for _ in 0..self.config.max_parallel_requests {
                cx.delay(SyncNext, None)?;
            }
            Ok(SyncerState {
                gaps: Gaps(sync_data),
                solidifiers: solidifiers.into_boxed_slice(),
            })
        } else {
            cx.shutdown();
            Ok(SyncerState::default())
        }
    }
}

#[async_trait]
impl HandleEvent<Report<Solidifier>> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Solidifier>,
        state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    SolidifierError::MissingStardustInxRequester => {
                        let actor_id = report.actor.id;
                        state.solidifiers[actor_id] = cx.spawn_child(report.actor).await;
                    }
                    _ => {
                        cx.shutdown();
                    }
                },
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

pub struct SyncNext;

#[async_trait]
impl HandleEvent<SyncNext> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        _evt: SyncNext,
        state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if let Some(milestone_index) = state.gaps.next() {
            if !self.is_synced(milestone_index).await? {
                log::info!("Requesting unsynced milestone {}.", milestone_index);
                match self
                    .inx_client
                    .read_milestone(inx::proto::MilestoneRequest {
                        milestone_index,
                        milestone_id: None,
                    })
                    .await
                {
                    Ok(milestone) => {
                        match MilestoneRecord::try_from(milestone.into_inner()) {
                            Ok(rec) => {
                                self.db.upsert_milestone_record(&rec).await?;
                                let (sender, receiver) = tokio::sync::oneshot::channel();
                                // Get or create the milestone state
                                let mut ms_state = MilestoneState::requested(rec.milestone_index, sender);
                                ms_state
                                    .process_queue
                                    .extend(Vec::from(rec.payload.essence.parents).into_iter());
                                state
                                    .solidifiers
                                    // Divide solidifiers fairly by milestone
                                    .get(rec.milestone_index as usize % self.config.solidifier_count)
                                    // Unwrap: We never remove solidifiers, so they should always exist
                                    .unwrap()
                                    .send(ms_state)?;
                                let handle = cx.handle().clone();
                                // Spawn a task to await the solidification
                                tokio::spawn(async move {
                                    receiver.await.ok();
                                    // Once solidification is complete, we can continue with this range.
                                    handle.send(SyncNext).ok();
                                });
                            }
                            Err(e) => {
                                log::error!("Could not read milestone: {:?}", e);
                                cx.delay(SyncNext, None)?;
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("No milestone response for {}: {}", milestone_index, e);
                        cx.delay(SyncNext, None)?;
                    }
                }
            }
        } else {
            log::info!("Sync complete");
            cx.shutdown();
        }
        Ok(())
    }
}
