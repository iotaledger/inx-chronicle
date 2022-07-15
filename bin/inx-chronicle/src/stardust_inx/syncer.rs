// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::VecDeque, ops::RangeInclusive, time::Duration};

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, HandleEvent, Report},
    types::tangle::MilestoneIndex,
};
use inx::{client::InxClient, tonic::Channel};

use super::{InxError, LedgerUpdateStream};

// The Syncer starts at a certain milestone index in the past and moves forwards in time trying to sync as many
// milestones as possible - including their cones.
pub struct Syncer {
    gaps: Gaps,
    db: MongoDb,
    inx_client: InxClient<Channel>,
}

impl Syncer {
    pub fn new(gaps: Vec<RangeInclusive<MilestoneIndex>>, db: MongoDb, inx_client: InxClient<Channel>) -> Self {
        Self {
            gaps: Gaps(gaps.into()),
            db,
            inx_client,
        }
    }
}

#[derive(Debug, Default)]
pub struct Gaps(VecDeque<RangeInclusive<MilestoneIndex>>);

impl Iterator for Gaps {
    type Item = RangeInclusive<MilestoneIndex>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(range) = self.0.pop_front() {
            if range.start() <= range.end() {
                return Some(range);
            }
        }
        None
    }
}

#[async_trait]
impl Actor for Syncer {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Syncer".into()
    }
}

#[async_trait]
impl HandleEvent<Report<LedgerUpdateStream>> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<LedgerUpdateStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // Start syncing the next milestone range
        cx.delay(SyncNext, None)?;
        match event {
            Report::Success(_) => (),
            Report::Error(report) => match report.error {
                ActorError::Result(e) => {
                    Err(e)?;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.abort().await;
                }
            },
        }
        Ok(())
    }
}

pub struct SyncNext;

#[async_trait]
impl HandleEvent<SyncNext> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        _evt: SyncNext,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if let Some(milestone_range) = self.gaps.next() {
            log::info!(
                "Requesting unsynced milestone range {}..{}.",
                milestone_range.start(),
                milestone_range.end()
            );
            cx.spawn_child(LedgerUpdateStream::new(
                self.db.clone(),
                self.inx_client.clone(),
                *milestone_range.start()..=*milestone_range.end(),
            ))
            .await;
        } else {
            log::info!("Successfully finished synchronization with node.");
            cx.shutdown().await;
        }
        Ok(())
    }
}

pub struct SyncProgress(pub Duration);

#[async_trait]
impl HandleEvent<SyncProgress> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        SyncProgress(delay): SyncProgress,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let first = self.db.find_first_milestone(0.into()).await?;
        let last = self.db.get_latest_milestone().await?;
        if let (Some(first), Some(last)) = (first, last) {
            let sync_data = self
                .db
                .get_sync_data(first.milestone_index..=last.milestone_index)
                .await?;
            log::info!("{:#}", sync_data);
            if !sync_data.gaps.is_empty() {
                cx.delay(SyncProgress(delay), delay)?;
            }
        }
        Ok(())
    }
}
