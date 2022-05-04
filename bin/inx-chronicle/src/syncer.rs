// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

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

const CHUNK_SIZE_DEFAULT: u32 = 50;
const COOLDOWN_TIME_DEFAULT: Duration = Duration::from_secs(300);

pub(crate) type MilestoneIndex = u32;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SyncerError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub(crate) struct Syncer {
    db: MongoDb,
    // TODO
    start_index: u32,
    // the number of milestones the syncer tries to retrieve in one go.
    chunk_size: u32,
    // the time the syncer waits before syncing the next batch of milestones.
    cooldown_time: Duration,
}

impl Syncer {
    pub(crate) fn new(db: MongoDb, start_index: u32) -> Self {
        Self {
            db,
            start_index,
            chunk_size: CHUNK_SIZE_DEFAULT,
            cooldown_time: COOLDOWN_TIME_DEFAULT,
        }
    }

    pub(crate) fn with_advancement_range(mut self, value: u32) -> Self {
        self.chunk_size = value;
        self
    }

    pub(crate) fn with_cooldown_time(mut self, value: Duration) -> Self {
        self.cooldown_time = value;
        self
    }
}

#[async_trait]
impl Actor for Syncer {
    type State = u32;
    type Error = SyncerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Syncer starting at {}", self.start_index);

        Ok(self.start_index)
    }
}

//
#[async_trait]
impl HandleEvent<MilestoneIndex> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        target_index: MilestoneIndex,
        current_index: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // TODO: impl cooldown
        if target_index < *current_index + self.chunk_size {
            Ok(())
        } else {
            let start_index = *current_index;
            let stop_index = start_index + self.chunk_size;

            log::info!("Syncing range [{}:{}]", start_index, stop_index);

            for index in start_index..stop_index {
                let synced_ms = self.db.get_sync_record_by_index(index).await;
                match synced_ms {
                    Ok(doc) => match doc {
                        Some(doc) => {
                            let sync_record: SyncRecord = bson::from_document(doc).expect("sync record from document");
                            if !sync_record.synced {
                                log::info!("Syncing {}.", index);
                                cx.addr::<InxWorker>().await.send(index)?;
                            } else {
                                log::info!("{index} already synced.");
                            }
                        }
                        None => {
                            log::info!("Syncing {}.", index);
                            cx.addr::<InxWorker>().await.send(index)?;
                        }
                    },
                    Err(e) => log::error!("{:?}", e),
                }
            }
            *current_index = stop_index;
            Ok(())
        }
    }
}
