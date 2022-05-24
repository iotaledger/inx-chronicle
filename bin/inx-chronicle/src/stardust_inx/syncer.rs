// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::VecDeque, ops::RangeInclusive};

use async_trait::async_trait;
use chronicle::{
    db::{model::tangle::MilestoneIndex, MongoDb},
    runtime::{Actor, ActorContext, ActorError, ConfigureActor, HandleEvent, Report},
};
use inx::{client::InxClient, tonic::Channel};

use super::{InxError, MilestoneStream};

// The Syncer goes backwards in time and tries collect as many milestones as possible.
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
}

#[async_trait]
impl HandleEvent<Report<MilestoneStream>> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MilestoneStream>,
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
            let milestone_stream = self
                .inx_client
                .listen_to_confirmed_milestones(inx::proto::MilestoneRangeRequest::from(
                    *milestone_range.start()..=*milestone_range.end(),
                ))
                .await?
                .into_inner();
            cx.spawn_child(
                MilestoneStream::new(self.db.clone(), self.inx_client.clone()).with_stream(milestone_stream),
            )
            .await;
        } else {
            log::info!("Sync complete");
            cx.shutdown();
        }
        Ok(())
    }
}
