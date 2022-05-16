// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{BTreeMap, HashSet},
    sync::atomic::{AtomicUsize, Ordering},
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
use inx::NodeStatus;
use serde::{Deserialize, Serialize};

use crate::inx::{InxRequest, InxWorker};

// solidifying a milestone must never take longer than the coordinator milestone interval
const MAX_SYNC_TIME: Duration = Duration::from_secs(10);

// TODO: remove
static NUM_REQUESTED: AtomicUsize = AtomicUsize::new(0);
static NUM_SYNCED: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, thiserror::Error)]
pub enum InxSyncerError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Bson(#[from] mongodb::bson::de::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InxSyncerConfig {
    // the maximum number of simultaneously open requests for milestones
    pub(crate) max_simultaneous_requests: usize,
    // the maximum number of requests for a single milestone
    pub(crate) max_request_retries: usize,
    // the number of historic milestones the Syncer tries to sync from the ledger index at start
    pub(crate) sync_back_delta: u32,
    // the fixed milestone index of a historic milestone the Syncer tries to sync back to
    pub(crate) sync_back_index: u32, // if set != 0 in the config, will override any also configured delta
}

impl Default for InxSyncerConfig {
    fn default() -> Self {
        Self {
            max_simultaneous_requests: 10,
            max_request_retries: 3,
            sync_back_delta: 10000,
            sync_back_index: 0,
        }
    }
}

// The Syncer goes backwards in time and tries collect as many milestones as possible.
#[derive(Debug)]
pub struct InxSyncer {
    db: MongoDb,
    config: InxSyncerConfig,
    internal_state: Option<SyncerState>,
}

impl InxSyncer {
    pub fn new(db: MongoDb, config: InxSyncerConfig) -> Self {
        Self {
            db,
            config,
            internal_state: None,
        }
    }

    pub fn with_internal_state(mut self, internal_state: SyncerState) -> Self {
        self.internal_state.replace(internal_state);
        self
    }

    async fn is_synced(&self, index: u32) -> Result<bool, InxSyncerError> {
        let sync_record = self.db.get_sync_record_by_index(index).await?;
        Ok(sync_record.map_or(false, |rec| rec.synced))
    }

    fn get_start_ms_index(&self, index: u32, sync_state: &mut SyncerState) -> u32 {
        // if the user specified a concrete sync start index then ignore
        // the `sync_back_delta` configuration.
        if self.config.sync_back_index != 0 {
            self.config.sync_back_index.max(sync_state.start_ms_index)
        } else if self.config.sync_back_delta != 0 {
            index
                .checked_sub(self.config.sync_back_delta)
                .unwrap_or(1)
                .max(sync_state.start_ms_index)
        } else {
            // Sync from the pruning index
            sync_state.start_ms_index
        }
    }
}

struct NextMilestoneToRequest(u32);
pub(crate) struct NewSyncedMilestone(pub(crate) u32);
pub(crate) struct NewTargetMilestone(pub(crate) u32);

#[derive(Debug, Default)]
pub struct SyncerState {
    // the oldest known milestone - usually the pruning index of the connected node
    start_ms_index: u32,
    // the index up to which the syncer should request milestones from the node
    target_ms_index: u32,
    // the set of milestones we are currently trying to sync
    pending: HashSet<u32>,
    // the set of milestones that we failed to sync on the 1st attempt and need to be retried
    retrying: BTreeMap<u32, usize>,
}

#[async_trait]
impl Actor for InxSyncer {
    type State = SyncerState;
    type Error = InxSyncerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        // Send a `NodeStatus` request to the `InxWorker`
        cx.addr::<InxWorker>().await.send(InxRequest::NodeStatus)?;

        let internal_state = self.internal_state.take();
        Ok(internal_state.unwrap_or_default())
    }
}

// issues requests in a controlled way
#[async_trait]
impl HandleEvent<NextMilestoneToRequest> for InxSyncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        NextMilestoneToRequest(index): NextMilestoneToRequest,
        syncer_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if index <= syncer_state.target_ms_index {
            if syncer_state.pending.len() < self.config.max_simultaneous_requests {
                if !self.is_synced(index).await? {
                    log::info!("Requesting historic milestone '{}'.", index);
                    cx.addr::<InxWorker>()
                        .await
                        .send(InxRequest::milestone(index.into(), cx.addr().await))?;
                    // sync_state.pending.insert(index, Instant::now());
                    syncer_state.pending.insert(index);
                    NUM_REQUESTED.fetch_add(1, Ordering::Relaxed);
                }
                cx.delay(NextMilestoneToRequest(index + 1), None)?;
            } else {
                cx.delay(NextMilestoneToRequest(index), Duration::from_secs_f32(0.5))?;
            }
        } else {
            log::info!("Syncer finished.");
        }
        // else if !sync_state.failed.is_empty() && sync_state.retry_round < self.config.max_request_retries {
        //     sync_state.retry_round += 1;
        //     log::info!(
        //         "Retrying {} failed requests (round #{})...",
        //         sync_state.failed.len(),
        //         sync_state.retry_round
        //     );
        //     let first_failed = sync_state.failed.iter().next().copied().unwrap();
        //     cx.delay(Request(first_failed), None)?;
        // }
        Ok(())
    }
}

// // updates the sync state with the latest milestone from the listening stream
// #[async_trait]
// impl HandleEvent<LatestMilestone> for Syncer {
//     async fn handle_event(
//         &mut self,
//         cx: &mut ActorContext<Self>,
//         LatestMilestone(index): LatestMilestone,
//         sync_state: &mut Self::State,
//     ) -> Result<(), Self::Error> {
//         println!(
//             "Requested: {}, Synced: {}",
//             NUM_REQUESTED.load(Ordering::Relaxed),
//             NUM_SYNCED.load(Ordering::Relaxed)
//         );

//         // Mark all milestones that are pending for too long as failed
//         // NOTE: some still unstable API like `drain_filter` would make this code much nicer!
//         let now = Instant::now();
//         for (index, timestamp) in sync_state.pending.iter() {
//             if now > *timestamp + MAX_SYNC_TIME {
//                 sync_state.failed.insert(*index);
//             }
//         }
//         sync_state.pending.retain(|_, t| now <= *t + MAX_SYNC_TIME);

//         Ok(())
//     }
// }

#[async_trait]
impl HandleEvent<NodeStatus> for InxSyncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        node_status: NodeStatus,
        syncer_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!(
            "Syncer received node status (pruning index = '{}').",
            node_status.pruning_index
        );

        syncer_state.start_ms_index = node_status.pruning_index + 1;
        log::debug!("Syncer determined start index: '{}'", syncer_state.start_ms_index);
        Ok(())
    }
}

// removes successfully synced milestones from `pending` or `retrying`
#[async_trait]
impl HandleEvent<NewSyncedMilestone> for InxSyncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        NewSyncedMilestone(latest_synced_index): NewSyncedMilestone,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Syncer received new synced milestone '{}'", latest_synced_index);

        let was_requested = sync_state.pending.remove(&latest_synced_index)
            || sync_state.retrying.remove(&latest_synced_index).is_some();
        if was_requested {
            // TODO: remove
            NUM_SYNCED.fetch_add(1, Ordering::Relaxed);
        } else if sync_state.target_ms_index == 0 {
            // Set the target to the first synced milestone that was not requested by the Syncer.
            sync_state.target_ms_index = latest_synced_index;
            log::debug!("Syncer determined target index: '{}'", sync_state.target_ms_index);

            log::info!(
                "Start syncing milestone range: [{}:{}]",
                sync_state.start_ms_index,
                sync_state.target_ms_index
            );
            cx.delay(NextMilestoneToRequest(sync_state.start_ms_index), None)?;
        }
        Ok(())
    }
}

// allows to resume the syncer
#[async_trait]
impl HandleEvent<NewTargetMilestone> for InxSyncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        NewTargetMilestone(new_target_ms_index): NewTargetMilestone,
        syncer_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Syncer received new target milestone '{}'", new_target_ms_index);

        if new_target_ms_index > syncer_state.target_ms_index {
            let previous_target = syncer_state.target_ms_index;
            syncer_state.target_ms_index = new_target_ms_index;
            // trigger the syncer again from the previous target index to the new
            let start_index = previous_target + 1;
            log::info!(
                "Start syncing milestone range: [{}:{}]",
                start_index,
                new_target_ms_index
            );
            cx.delay(NextMilestoneToRequest(start_index), None)?;
        }
        Ok(())
    }
}
