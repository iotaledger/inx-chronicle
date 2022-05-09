// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{time::Duration, ops::Range};

use async_trait::async_trait;
use chronicle::{
    db::{model::sync::SyncRecord, MongoDb},
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use mongodb::bson;
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
    #[serde(default, with = "humantime_serde")]
    pub(crate) cooldown: Duration,
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

    async fn collect_unsolid_milestones(&self, start_index: u32, end_index: u32) -> Result<Vec<u32>, SyncerError> {
        debug_assert!(end_index >= start_index);
        log::info!("Searching for unsolid milestones in [{}:{}]...", start_index, end_index);

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
                    log::debug!("Found {} unsolid milestones.", unsolid_milestones.len());
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
impl HandleEvent<Range<u32>> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        range: Range<u32>,
        _: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let unsolid_milestones = self.collect_unsolid_milestones(range.start, range.end).await?;

        // let mut num_requested = 0;
        for index in unsolid_milestones.into_iter() {
            log::info!("Requesting milestone {}.", index);
            cx.addr::<InxWorker>().await.send(InxRequest::milestone(index.into()))?;

            // Cooldown a bit before issuing the next request.
            tokio::time::sleep(self.config.cooldown).await;
        }

        Ok(())
    }
}
