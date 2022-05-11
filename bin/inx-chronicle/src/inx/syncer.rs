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

use super::InxWorkerError;
use crate::inx::{InxWorker, MilestoneRequest, NodeStatusRequest};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SyncerError {
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Request(#[from] InxWorkerError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncerConfig {
    pub(crate) max_milestones: u32,
    #[serde(with = "humantime_serde")]
    pub(crate) rate_limit: Duration,
}

impl Default for SyncerConfig {
    fn default() -> Self {
        Self {
            max_milestones: 10000,
            rate_limit: Duration::from_millis(500),
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

pub(crate) struct SyncRange {
    start: u32,
    end: u32,
}

impl SyncRange {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
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
impl HandleEvent<SyncRange> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        SyncRange { start, end }: SyncRange,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let target_time = Instant::now() + self.config.rate_limit;
        if start <= end {
            if !self.is_synced(start).await? {
                log::info!("Requesting unsolid milestone {}.", start);
                cx.addr::<InxWorker>().await.send(MilestoneRequest::new(start, 3))?;
                tokio::time::sleep_until(target_time).await;
            }
            cx.delay(SyncRange::new(start + 1, end), None)?;
        } else {
            log::info!("Syncer completed range.");
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
            node_status
                .pruning_index
                .max(node_status.latest_milestone.milestone_index - self.config.max_milestones),
            node_status.latest_milestone.milestone_index,
        );
        let gaps = self.db.get_sync_data(start, end).await?.gaps;
        for gap in gaps {
            log::debug!("Requesting gap {} to {}", gap.start, gap.end);
            // Get the overlap between the gap and our syncing range
            cx.delay(SyncRange::new(gap.start, gap.end), None)?;
        }
        Ok(())
    }
}
