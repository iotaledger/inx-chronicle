// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

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

use crate::inx::{InxWorker, MilestoneRequest};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SyncerError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Bson(#[from] mongodb::bson::de::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncerConfig {
    pub(crate) earliest_milestone: u32,
    #[serde(with = "humantime_serde")]
    pub(crate) rate_limit: Duration,
}

impl Default for SyncerConfig {
    fn default() -> Self {
        Self {
            earliest_milestone: 0,
            rate_limit: Duration::from_millis(100),
        }
    }
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

    async fn is_synced(&self, index: u32) -> Result<bool, SyncerError> {
        let sync_record = self.db.get_sync_record_by_index(index).await?;
        Ok(sync_record.map_or(false, |rec| rec.synced))
    }
}

pub(crate) struct SyncRange {
    pub(crate) start: Option<u32>,
    pub(crate) end: u32,
}

#[async_trait]
impl Actor for Syncer {
    type State = ();
    type Error = SyncerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<SyncRange> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        SyncRange { start, end }: SyncRange,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if start.is_none() {
            log::debug!(
                "Received ending milestone, syncing from {} to {}.",
                self.config.earliest_milestone,
                end
            );
        }
        let start = start.unwrap_or(self.config.earliest_milestone);
        let target_time = Instant::now() + self.config.rate_limit;
        if start < end {
            if !self.is_synced(start).await? {
                log::info!("Requesting unsolid milestone {}.", start);
                cx.addr::<InxWorker>().await.send(MilestoneRequest::new(start))?;
            }
            tokio::time::sleep_until(target_time).await;
            cx.delay(
                SyncRange {
                    start: Some(start + 1),
                    end,
                },
                None,
            )?;
        } else {
            log::info!("Syncer completed.");
        }
        Ok(())
    }
}
