// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashSet, time::Duration};

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use serde::{Deserialize, Serialize};

use crate::inx::{InxRequest, InxWorker};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SyncerError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Bson(#[from] mongodb::bson::de::Error),
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncerConfig {
    pub(crate) max_simultaneous_requests: usize,
    pub(crate) max_milestones_to_sync: u32,
}
// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub(crate) struct Syncer {
    db: MongoDb,
    #[allow(dead_code)]
    config: SyncerConfig,
}

impl Syncer {
    pub(crate) fn new(db: MongoDb, config: SyncerConfig) -> Self {
        Self { db, config }
    }

    async fn is_synced(&self, index: u32) -> Result<bool, SyncerError> {
        let sync_record = self.db.get_sync_record_by_index(index).await?;
        Ok(sync_record.map_or(false, |rec| rec.synced))
    }
}

struct Next(u32);
pub(crate) struct LatestMilestone(pub(crate) u32);
pub(crate) struct OldestMilestone(pub(crate) u32);
pub(crate) struct LatestSolidified(pub(crate) u32);
pub(crate) struct TargetMilestone(pub(crate) u32);

#[derive(Default)]
pub(crate) struct SyncState {
    // the oldest known milestone - usually the pruning index of the connected node
    oldest_milestone: u32,
    // the index up to which the syncer should request milestones from the node
    target_milestone: u32,
    // the oldest milestones we were able to sync
    oldest_synced_milestone: u32,
    // the latest milestone we know of from listening to the milestone stream
    latest_milestone: u32,
    // the set of milestones we are currently trying to sync
    pending: HashSet<u32>,
}

#[async_trait]
impl Actor for Syncer {
    type State = SyncState;
    type Error = SyncerError;

    async fn init(&mut self, _: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(SyncState::default())
    }
}

// issues requests in a controlled way
#[async_trait]
impl HandleEvent<Next> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        Next(index): Next,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if index <= sync_state.target_milestone {
            if sync_state.pending.len() < self.config.max_simultaneous_requests {
                if !self.is_synced(index).await? {
                    log::info!("Requesting unsolid milestone {}.", index);
                    cx.addr::<InxWorker>().await.send(InxRequest::milestone(index.into()))?;
                    sync_state.pending.insert(index);
                }
                cx.delay(Next(index + 1), None)?;
            } else {
                // wait a bit and try again
                // TODO: can we assume that `pending` always decreases over time?
                cx.delay(Next(index), Duration::from_secs_f32(0.01))?;
            }
        } else {
            log::info!("Syncer completed.")
        }
        Ok(())
    }
}

// sets the oldest milestone the syncer can try to sync from if the user configured to do so
#[async_trait]
impl HandleEvent<OldestMilestone> for Syncer {
    async fn handle_event(
        &mut self,
        _: &mut ActorContext<Self>,
        OldestMilestone(index): OldestMilestone,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        sync_state.oldest_milestone = sync_state.oldest_milestone.min(index);
        Ok(())
    }
}

// changes the target milestone of the syncer moving the upper bound of the syncing range
#[async_trait]
impl HandleEvent<TargetMilestone> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        TargetMilestone(index): TargetMilestone,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if index > sync_state.target_milestone {
            let previous_target = sync_state.target_milestone;
            sync_state.target_milestone = index;
            // trigger the syncer again from the previous target index to the new
            cx.delay(Next(previous_target + 1), None)?;
        }
        Ok(())
    }
}

// updates the sync state with the latest milestone from the listening stream
#[async_trait]
impl HandleEvent<LatestMilestone> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        LatestMilestone(index): LatestMilestone,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // First ever listened milestone? Get the start index and trigger syncing.
        if sync_state.latest_milestone == 0 {
            sync_state.target_milestone = index;
            sync_state.latest_milestone = index;
            let index = if self.config.max_milestones_to_sync != 0 {
                index
                    .checked_sub(self.config.max_milestones_to_sync)
                    .unwrap_or(1)
                    .max(sync_state.oldest_milestone)
            } else {
                // Sync from the pruning index.
                sync_state.oldest_milestone
            };
            // Actually triggers the Syncer.
            cx.delay(Next(index), None)?;
        } else if index != sync_state.latest_milestone + 1 {
            log::warn!("Latest milestone isn't the direct successor of the previous one.");
            sync_state.latest_milestone = index;
        }
        Ok(())
    }
}

// potentially makes room for another INX milestone request
#[async_trait]
impl HandleEvent<LatestSolidified> for Syncer {
    async fn handle_event(
        &mut self,
        _: &mut ActorContext<Self>,
        LatestSolidified(index): LatestSolidified,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        sync_state.oldest_synced_milestone = sync_state.oldest_synced_milestone.min(index);
        sync_state.pending.remove(&index);
        Ok(())
    }
}
