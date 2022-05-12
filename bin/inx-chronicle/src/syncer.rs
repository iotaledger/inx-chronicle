// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{BTreeSet, HashMap},
    time::Duration,
};

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::inx::{InxRequest, InxWorker};

// solidifying a milestone must never take longer than the coordinator milestone interval
const MAX_SYNC_TIME: Duration = Duration::from_secs(10);

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
    // the maximum number of simultaneously open requests for milestones
    pub(crate) max_simultaneous_requests: usize,
    // the maximum number of requests for a single milestone
    pub(crate) max_request_retries: usize,
    // the number of historic milestones the Syncer tries to sync from the ledger index at start
    pub(crate) sync_back_delta: u32,
    // the fixed milestone index of a historic milestone the Syncer tries to sync back to
    pub(crate) sync_back_index: u32, // if set != 0 in the config, will override any also configured delta
}
// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub(crate) struct Syncer {
    db: MongoDb,
    #[allow(dead_code)]
    config: SyncerConfig,
    internal_state: Option<SyncState>,
}

impl Syncer {
    pub fn new(db: MongoDb, config: SyncerConfig) -> Self {
        Self {
            db,
            config,
            internal_state: None,
        }
    }

    pub fn with_internal_state(mut self, internal_state: SyncState) -> Self {
        self.internal_state.replace(internal_state);
        self
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
    pending: HashMap<u32, Instant>,
    // the set of milestones that we failed to sync on the 1st attempt and need to be retried
    failed: BTreeSet<u32>,
    // the current round of retrying requests that failed earlier
    retry_round: usize,
}

#[async_trait]
impl Actor for Syncer {
    type State = SyncState;
    type Error = SyncerError;

    async fn init(&mut self, _: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let internal_state = self.internal_state.take();
        Ok(internal_state.unwrap_or_default())
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
                    log::info!("Requesting old milestone {}.", index);
                    cx.addr::<InxWorker>().await.send(InxRequest::milestone(index.into()))?;
                    sync_state.pending.insert(index, Instant::now());
                }
                cx.delay(Next(index + 1), None)?;
            } else {
                // wait a bit and try again
                // TODO: can we assume that `pending` always decreases over time?
                cx.delay(Next(index), Duration::from_secs_f32(0.01))?;
            }
        } else if !sync_state.failed.is_empty() && sync_state.retry_round < self.config.max_request_retries {
            sync_state.retry_round += 1;
            log::info!(
                "Retrying {} failed requests (round #{})...",
                sync_state.failed.len(),
                sync_state.retry_round
            );
            // Grab the first failed index and start the Syncer again from that index.
            // Any other potential gaps will be retried as well, and the Syncer doesn't
            // need some 2nd mode of operation. It is less efficient, however, since
            // we again check the database for each milestone's sync state. Since the
            // Syncer does most of its work only one time (at startup), this "extra"
            // work is neglibible, and it's therefore better to keep the syncer as
            // simple as possible.
            // Panic: `failed` is not empty so a first item must exist.
            let first_failed = sync_state.failed.iter().next().copied().unwrap();
            cx.delay(Next(first_failed), None)?;
        } else {
            log::info!("Syncer finished (no milestones missing).");
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
            let start_index = previous_target + 1;
            log::info!("Syncing [{}:{}]", start_index, index);
            cx.delay(Next(start_index), None)?;
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
        // Mark all milestones that are pending for too long as failed
        // NOTE: some still unstable API like `drain_filter` would make this code much nicer!
        let now = Instant::now();
        for (index, timestamp) in sync_state.pending.iter() {
            if now > *timestamp + MAX_SYNC_TIME {
                sync_state.failed.insert(*index);
            }
        }
        sync_state.pending.retain(|_, t| now <= *t + MAX_SYNC_TIME);

        // First ever listened milestone? Get the start index and trigger syncing.
        if sync_state.latest_milestone == 0 {
            sync_state.latest_milestone = index;
            sync_state.target_milestone = index;

            // if the user specified a concrete sync start index then ignore
            // the `sync_back_delta` configuration.
            let start_index = if self.config.sync_back_index != 0 {
                self.config.sync_back_index.max(sync_state.oldest_milestone)
            } else if self.config.sync_back_delta != 0 {
                index
                    .checked_sub(self.config.sync_back_delta)
                    .unwrap_or(1)
                    .max(sync_state.oldest_milestone)
            } else {
                // Sync from the pruning index
                sync_state.oldest_milestone
            };

            // Actually start syncing
            log::info!("Syncing [{}:{}]", start_index, index);
            cx.delay(Next(start_index), None)?;
        } else if index > sync_state.latest_milestone {
            if index != sync_state.latest_milestone + 1 {
                log::warn!(
                    "Latest milestone isn't the direct successor of the previous one: {} {}.",
                    index,
                    sync_state.latest_milestone
                );
            }
            sync_state.latest_milestone = index;
        }
        Ok(())
    }
}

// potentially frees up slots for the next INX milestone request
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
        sync_state.failed.remove(&index);
        Ok(())
    }
}
