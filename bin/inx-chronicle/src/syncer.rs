// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use async_trait::async_trait;
use chronicle::{
    db::{model::sync::SyncRecord, MongoDb},
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use mongodb::bson;

use crate::inx::InxWorker;

const MIN_BATCH_SIZE: u32 = 1;
const MAX_BATCH_SIZE: u32 = 50;

pub(crate) type MilestoneIndex = u32;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SyncerError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub(crate) struct Syncer {
    db: MongoDb,
    // the index we start syncing from.
    start_index: u32,
    // the index we stop syncing.
    end_index: u32,
    // the batch of simultaneous synced milestones.
    batch_size: u32,
    // the requested milestone indexes.
    batch: HashSet<u32>,
}

impl Syncer {
    pub(crate) fn new(db: MongoDb, start_index: u32, end_index: u32) -> Self {
        Self {
            db,
            start_index,
            end_index,
            batch_size: 1,
            batch: HashSet::new(),
        }
    }

    pub(crate) fn with_batch_size(mut self, value: u32) -> Self {
        self.batch_size = value.max(MIN_BATCH_SIZE).min(MAX_BATCH_SIZE);
        self
    }
}

#[async_trait]
impl Actor for Syncer {
    type State = MilestoneIndex;
    type Error = SyncerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(self.start_index)
    }
}

#[async_trait]
impl HandleEvent<()> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        _: (),
        current_index: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if *current_index < self.start_index || *current_index > self.end_index {
            Ok(())
        } else {
            let start_index = *current_index + 1;
            let stop_index = start_index + self.batch_size;

            log::info!("Syncing range [{}:{}]", start_index, stop_index);

            let mut index = start_index;
            let mut num_requested = 0;
            'next_milestone: loop {
                let synced_ms = self.db.get_sync_record_by_index(index).await;
                match synced_ms {
                    Ok(doc) => match doc {
                        Some(doc) => {
                            let sync_record: SyncRecord = bson::from_document(doc).expect("sync record from document");
                            if !sync_record.synced {
                                log::info!("Requesting old milestone {}.", index);
                                cx.addr::<InxWorker>().await.send(index)?;
                                num_requested += 1;
                            } else {
                                log::info!("{index} already synced.");
                            }
                        }
                        None => {
                            log::info!("Syncing {}.", index);
                            cx.addr::<InxWorker>().await.send(index)?;
                            num_requested += 1;
                        }
                    },
                    Err(e) => log::error!("{:?}", e),
                }
                if num_requested == self.batch_size {
                    break 'next_milestone;
                }

                index += 1;
            }
            *current_index = index;
            Ok(())
        }
    }
}

#[async_trait]
impl HandleEvent<MilestoneIndex> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        synced_milestone_index: MilestoneIndex,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        self.batch.remove(&synced_milestone_index);

        // Only if the whole batch has been processed trigger a new sync.
        if self.batch.is_empty() {
            cx.addr::<Self>().await.send(())?;
        }

        Ok(())
    }
}
