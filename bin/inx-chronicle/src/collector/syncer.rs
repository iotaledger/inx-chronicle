// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use inx::NodeStatus;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::inx::{InxWorker, MilestoneRequest, NodeStatusRequest};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SyncerError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Bson(#[from] mongodb::bson::de::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncerConfig {
    pub(crate) earliest_milestone: u32,
    #[serde(with = "humantime_serde")]
    pub(crate) rate_limit: Duration,
}

impl Default for SyncerConfig {
    fn default() -> Self {
        Self {
            earliest_milestone: 0,
            rate_limit: Duration::from_millis(100),
        }
    }
}

// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub(crate) struct Syncer {
    db: MongoDb,
    config: SyncerConfig,
}

impl Syncer {
    pub(crate) fn new(db: MongoDb, config: SyncerConfig) -> Self {
        Self { db, config }
    }

    async fn is_synced(&self, index: u32) -> Result<bool, SyncerError> {
        Ok(self.db.get_sync_record_by_index(index).await?.is_some())
    }
}

pub(crate) enum SyncerEvent {
    SyncRange {
        start: u32,
        end: u32,
    },
    #[allow(dead_code)]
    Retry {
        milestone: u32,
    },
}

impl SyncerEvent {
    pub fn sync_range(start: u32, end: u32) -> Self {
        Self::SyncRange { start, end }
    }

    #[allow(dead_code)]
    pub fn retry(milestone: u32) -> Self {
        Self::Retry { milestone }
    }
}

#[async_trait]
impl Actor for Syncer {
    type State = ();
    type Error = SyncerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        cx.addr::<InxWorker>()
            .await
            .send(NodeStatusRequest::new(cx.handle().clone()))?;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<SyncerEvent> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: SyncerEvent,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            SyncerEvent::SyncRange { start, end } => {
                let target_time = Instant::now() + self.config.rate_limit;
                if start <= end {
                    if !self.is_synced(start).await? {
                        log::info!("Requesting unsolid milestone {}.", start);
                        cx.addr::<InxWorker>().await.send(MilestoneRequest::new(start))?;
                    }
                    tokio::time::sleep_until(target_time).await;
                    cx.delay(SyncerEvent::sync_range(start + 1, end), None)?;
                } else {
                    log::info!("Syncer completed range.");
                }
            }
            SyncerEvent::Retry { milestone } => {
                log::info!("Retrying milestone {}.", milestone);
                cx.addr::<InxWorker>().await.send(MilestoneRequest::new(milestone))?;
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
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // Get our maximum syncing range based on what we know from the node status
        let (start, end) = (
            node_status.pruning_index.max(self.config.earliest_milestone),
            node_status.latest_milestone.milestone_index,
        );
        let gaps = self.db.get_sync_data(start, end).await?.gaps;
        for gap in gaps {
            // Get the overlap between the gap and our syncing range
            cx.delay(SyncerEvent::sync_range(gap.start, gap.end), None)?;
        }
        Ok(())
    }
}
