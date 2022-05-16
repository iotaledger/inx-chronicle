// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{ops::Range, time::Duration};

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent, RuntimeError},
};
use inx::NodeStatus;
use serde::{Deserialize, Serialize};

use super::{
    error::InxWorkerError,
    worker::{InxWorker, MilestoneRequest, NodeStatusRequest},
};

#[derive(Debug, thiserror::Error)]
pub enum SyncerError {
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Request(#[from] InxWorkerError),
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
    #[serde(default, with = "humantime_serde")]
    pub retry_delay: Duration,
}

impl Default for SyncerConfig {
    fn default() -> Self {
        Self {
            sync_kind: Default::default(),
            max_parallel_requests: 10,
            retry_delay: Duration::from_secs(5),
        }
    }
}

// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub struct Syncer {
    db: MongoDb,
    config: SyncerConfig,
}

impl Syncer {
    pub fn new(db: MongoDb, config: SyncerConfig) -> Self {
        Self { db, config }
    }

    async fn is_synced(&self, index: u32) -> Result<bool, SyncerError> {
        Ok(self.db.get_sync_record_by_index(index).await?.is_some())
    }
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

pub struct SyncNext;
pub struct RequestNodeStatus;

#[async_trait]
impl Actor for Syncer {
    type State = Option<Gaps>;
    type Error = SyncerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        cx.addr::<InxWorker>()
            .await
            .send(NodeStatusRequest::new(cx.handle().clone()))?;
        Ok(Default::default())
    }
}

#[async_trait]
impl HandleEvent<SyncNext> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        _evt: SyncNext,
        gaps: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if let Some(gaps_iter) = gaps {
            if let Some(ms) = gaps_iter.next() {
                if !self.is_synced(ms).await? {
                    log::info!("Requesting unsynced milestone {}.", ms);
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    cx.addr::<InxWorker>().await.send(MilestoneRequest::new(ms, sender))?;
                    let handle = cx.handle().clone();
                    // Spawn a task to await the solidification
                    tokio::spawn(async move {
                        receiver.await.ok();
                        // Once solidification is complete, we can continue with this range.
                        handle.send(SyncNext).ok();
                    });
                }
            } else {
                *gaps = None;
                cx.delay(RequestNodeStatus, Some(self.config.retry_delay))?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<NodeStatus> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        node_status: NodeStatus,
        gaps: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // Get our maximum syncing range based on what we know from the node status
        let configured_start = match self.config.sync_kind {
            SyncKind::Max(ms) => node_status.latest_milestone.milestone_index - ms,
            SyncKind::From(ms) => ms,
        };
        let (start, end) = (
            node_status.pruning_index.max(configured_start),
            node_status.latest_milestone.milestone_index,
        );
        let sync_data = self.db.get_sync_data(start, end).await?.gaps;
        if sync_data.is_empty() {
            log::info!("Sync complete");
            cx.shutdown();
            return Ok(());
        }
        gaps.replace(Gaps(sync_data));
        for _ in 0..self.config.max_parallel_requests {
            cx.delay(SyncNext, None)?;
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<RequestNodeStatus> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        _evt: RequestNodeStatus,
        _gaps: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::info!("Requesting node status");
        cx.addr::<InxWorker>()
            .await
            .send(NodeStatusRequest::new(cx.handle().clone()))?;
        Ok(())
    }
}
