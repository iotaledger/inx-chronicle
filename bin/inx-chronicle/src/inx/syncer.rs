// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

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

use super::InxWorkerError;
use crate::inx::{worker::stardust::MilestoneRequest, InxWorker, NodeStatusRequest};

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
#[serde(default)]
pub struct SyncerConfig {
    pub max_milestones: u32,
    pub max_parallel_requests: usize,
}

impl Default for SyncerConfig {
    fn default() -> Self {
        Self {
            max_milestones: 10000,
            max_parallel_requests: 10,
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

#[derive(Default)]
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

pub struct SyncNext(usize);

#[async_trait]
impl Actor for Syncer {
    type State = Gaps;
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
        SyncNext(num): SyncNext,
        gaps: &mut Self::State,
    ) -> Result<(), Self::Error> {
        for _ in 0..num {
            if let Some(ms) = gaps.next() {
                if !self.is_synced(ms).await? {
                    log::info!("Requesting unsynced milestone {}.", ms);
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    cx.addr::<InxWorker>()
                        .await
                        .send(MilestoneRequest::new(ms, 3, sender))?;
                    let handle = cx.handle().clone();
                    // Spawn a task to await the solidification
                    tokio::spawn(async move {
                        receiver.await.ok();
                        // Once solidification is complete, we can continue with this range.
                        handle.send(SyncNext(1)).ok();
                    });
                }
            } else {
                log::info!("Syncer completed range.");
                break;
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
        let (start, end) = (
            node_status
                .pruning_index
                .max(node_status.latest_milestone.milestone_index - self.config.max_milestones),
            node_status.latest_milestone.milestone_index,
        );
        *gaps = Gaps(self.db.get_sync_data(start, end).await?.gaps);
        cx.delay(SyncNext(self.config.max_parallel_requests), None)?;
        Ok(())
    }
}
