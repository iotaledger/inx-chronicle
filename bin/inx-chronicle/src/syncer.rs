// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use async_trait::async_trait;
use bee_message_stardust::payload::milestone::MilestoneIndex;
use chronicle::{
    db::{model::sync::SyncRecord, MongoDb},
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use mongodb::bson;

use crate::inx::{InxRequest, InxWorker};

const MIN_BATCH_SIZE: usize = 1;
const MAX_BATCH_SIZE: usize = 50;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SyncerError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Bson(#[from] mongodb::bson::de::Error),
}

// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub(crate) struct Syncer {
    db: MongoDb,
    // the batch of simultaneous synced milestones.
    batch_size: usize,
}

impl Syncer {
    pub(crate) fn new(db: MongoDb) -> Self {
        Self { db, batch_size: 1 }
    }

    pub(crate) fn with_batch_size(mut self, value: usize) -> Self {
        self.batch_size = value.max(MIN_BATCH_SIZE).min(MAX_BATCH_SIZE);
        self
    }

    async fn collect_unsolid_milestones(&self, start_index: u32, end_index: u32) -> Result<Vec<u32>, SyncerError> {
        debug_assert!(end_index >= start_index);
        log::info!("Collecting unsolid milestones in [{}:{}]...", start_index, end_index);

        let mut unsolid_milestones = Vec::with_capacity((end_index - start_index) as usize);

        for index in start_index..end_index {
            let sync_record = self.db.get_sync_record_by_index(index).await?;
            if match sync_record {
                Some(doc) => {
                    let sync_record: SyncRecord = bson::from_document(doc).map_err(SyncerError::Bson)?;
                    !sync_record.synced
                }
                None => true,
            } {
                unsolid_milestones.push(index);

                if unsolid_milestones.len() % 1000 == 0 {
                    log::debug!("Missing {}", unsolid_milestones.len());
                }
            }
        }

        log::info!("{} unsynced milestones detected.", unsolid_milestones.len());

        Ok(unsolid_milestones)
    }
}

#[async_trait]
impl Actor for Syncer {
    type State = ();
    type Error = SyncerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        // Send a `NodeStatus` request to the `InxWorker`
        cx.addr::<InxWorker>().await.send(InxRequest::NodeStatus)?;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<(MilestoneIndex, MilestoneIndex)> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        (start_index, end_index): (MilestoneIndex, MilestoneIndex),
        _: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let unsolid_milestones = self.collect_unsolid_milestones(*start_index, *end_index).await?;

        let mut num_requested = 0;
        for index in unsolid_milestones.into_iter() {
            log::info!("Requesting milestone {}.", index);
            cx.addr::<InxWorker>().await.send(InxRequest::Milestone(index.into()))?;

            num_requested += 1;

            if num_requested == self.batch_size {
                // FIXME: solidifying is super slow atm so we have this high cooldown.
                tokio::time::sleep(Duration::from_secs(60)).await;
                num_requested = 0;
            }
        }

        Ok(())
    }
}
