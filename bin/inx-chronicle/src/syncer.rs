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

#[derive(Default)]
pub(crate) struct SyncState {
    earliest_milestone: u32,
    first_synced_milestone: u32,
    latest_milestone: u32, // inclusive
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

#[async_trait]
impl HandleEvent<Next> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        Next(index): Next,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // let index = sync_state.next;
        if index <= sync_state.latest_milestone {
            if sync_state.pending.len() < self.config.max_simultaneous_requests {
                if !self.is_synced(index).await? {
                    log::info!("Requesting unsolid milestone {}.", index);
                    cx.addr::<InxWorker>().await.send(InxRequest::milestone(index.into()))?;
                    sync_state.pending.insert(index);
                }
                if index < sync_state.latest_milestone {
                    cx.delay(Next(index + 1), None)?;
                }
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

#[async_trait]
impl HandleEvent<OldestMilestone> for Syncer {
    async fn handle_event(
        &mut self,
        _: &mut ActorContext<Self>,
        OldestMilestone(index): OldestMilestone,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        sync_state.earliest_milestone = sync_state.earliest_milestone.min(index);
        Ok(())
    }
}

// that moves the upper bound of the sync range
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
            sync_state.latest_milestone = index;
            let next = if self.config.max_milestones_to_sync != 0 {
                index.checked_sub(self.config.max_milestones_to_sync).unwrap_or(1)
            } else {
                // Sync from the pruning index.
                sync_state.earliest_milestone
            };
            // Actually triggers the Syncer.
            cx.delay(Next(next), None)?;
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
        sync_state.first_synced_milestone = sync_state.first_synced_milestone.min(index);
        sync_state.pending.remove(&index);
        Ok(())
    }
}
