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

    async fn is_unsolid(&self, index: u32) -> Result<bool, SyncerError> {
        let sync_record = self.db.get_sync_record_by_index(index).await?;
        Ok(sync_record.map_or(true, |rec| !rec.synced))
    }
}

pub(crate) struct Next(pub(crate) u32);
pub(crate) struct Latest(pub(crate) u32);
pub(crate) struct Run;
pub(crate) struct Solidified(pub(crate) u32);

#[derive(Default)]
pub(crate) struct SyncState {
    next: u32,
    latest: u32, // inclusive
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
impl HandleEvent<Run> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        _: Run,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let index = sync_state.next;
        if sync_state.pending.len() < self.config.max_simultaneous_requests {
            if self.is_unsolid(index).await? {
                log::info!("Requesting unsolid milestone {}.", index);
                cx.addr::<InxWorker>().await.send(InxRequest::milestone(index.into()))?;
                sync_state.pending.insert(index);
            }
            if index < sync_state.latest {
                cx.addr::<Syncer>().await.send(Next(index + 1))?;
            }
        } else {
            // wait a bit and try again
            // TODO: can we assume that `pending` always decreases over time?
            tokio::time::sleep(Duration::from_secs_f32(0.01)).await;
            cx.addr::<Syncer>().await.send(Next(index))?;
        }

        Ok(())
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
        sync_state.next = index;
        if sync_state.next <= sync_state.latest {
            cx.addr::<Syncer>().await.send(Run)?;
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Latest> for Syncer {
    async fn handle_event(
        &mut self,
        _: &mut ActorContext<Self>,
        Latest(index): Latest,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if index != sync_state.latest + 1 {
            log::warn!("Latest milestone didn't isn't the direct successor of the previous one.");
        }
        sync_state.latest = index;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Solidified> for Syncer {
    async fn handle_event(
        &mut self,
        _: &mut ActorContext<Self>,
        Solidified(index): Solidified,
        sync_state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        sync_state.pending.remove(&index);
        Ok(())
    }
}
