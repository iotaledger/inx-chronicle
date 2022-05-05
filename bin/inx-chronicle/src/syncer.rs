// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use async_trait::async_trait;
use chronicle::{
    db::{model::sync::SyncRecord, MongoDb},
    runtime::{
        actor::{addr::Addr, context::ActorContext, event::HandleEvent, Actor, report::Report},
        config::ConfigureActor,
        error::RuntimeError,
    },
};
use mongodb::bson;

use crate::{collector::solidifier::Solidifier, inx::InxWorker};

const MIN_BATCH_SIZE: usize = 1;
const MAX_BATCH_SIZE: usize = 50;

pub(crate) type MilestoneIndex = u32;

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
    // the index we start syncing from.
    start_index: MilestoneIndex,
    // the index we stop syncing.
    end_index: MilestoneIndex,
    // the batch of simultaneous synced milestones.
    batch_size: usize,
    // the requested milestone indexes.
    milestones_to_sync: Vec<MilestoneIndex>,
}

impl Syncer {
    pub(crate) fn new(db: MongoDb, start_index: MilestoneIndex, end_index: MilestoneIndex) -> Self {
        debug_assert!(end_index >= start_index);

        Self {
            db,
            start_index,
            end_index,
            batch_size: 1,
            milestones_to_sync: Vec::with_capacity((end_index - start_index) as usize),
        }
    }

    pub(crate) fn with_batch_size(mut self, value: usize) -> Self {
        self.batch_size = value.max(MIN_BATCH_SIZE).min(MAX_BATCH_SIZE);
        self
    }

    async fn collect_milestone_gaps(&mut self) -> Result<(), SyncerError> {
        for index in self.start_index..self.end_index {
            let sync_record = self.db.get_sync_record_by_index(index).await?;
            if match sync_record {
                Some(doc) => {
                    let sync_record: SyncRecord = bson::from_document(doc).map_err(SyncerError::Bson)?;
                    !sync_record.synced
                }
                None => true,
            } {
                self.milestones_to_sync.push(index);
            }
        }

        log::info!("{} unsynced milestones detected.", self.milestones_to_sync.len());

        Ok(())
    }
}

#[async_trait]
impl Actor for Syncer {
    type State = ();
    type Error = SyncerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        cx.shutdown();

        self.collect_milestone_gaps().await?;

        let mut num_requested = 0;
        for index in self.milestones_to_sync.iter().copied() {
            log::info!("Requesting milestone {}.", index);
            cx.addr::<InxWorker>().await.send(index)?;

            num_requested += 1;

            if num_requested == self.batch_size {
                tokio::time::sleep(Duration::from_secs(1)).await;
                num_requested = 0;
            }
        }

        Ok(())
    }
}
