// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use async_trait::async_trait;
use chronicle::{
    db::{model::stardust::milestone::MilestoneRecord, MongoDb},
    runtime::{Actor, ActorContext, ActorError, ConfigureActor, HandleEvent, Report},
};
use inx::{client::InxClient, tonic::Channel};

use super::{cone_stream::ConeStream, InxError};

// The Syncer goes backwards in time and tries collect as many milestones as possible.
pub struct Syncer {
    gaps: Gaps,
    db: MongoDb,
    inx_client: InxClient<Channel>,
}

impl Syncer {
    pub fn new(gaps: Vec<Range<u32>>, db: MongoDb, inx_client: InxClient<Channel>) -> Self {
        Self {
            gaps: Gaps(gaps),
            db,
            inx_client,
        }
    }
}

#[derive(Debug, Default)]
pub struct Gaps(Vec<Range<u32>>);

impl Iterator for Gaps {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(range) = self.0.first() {
            if range.start >= range.end {
                self.0.remove(0);
            } else {
                break;
            }
        }
        if let Some(range) = self.0.first_mut() {
            let next = range.start;
            range.start += 1;
            Some(next)
        } else {
            None
        }
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
impl HandleEvent<Report<ConeStream>> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<ConeStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // Start syncing the next milestone
        cx.delay(SyncNext, None)?;
        match event {
            Report::Success(_) => (),
            Report::Error(e) => match e.error {
                ActorError::Result(e) => {
                    Err(e)?;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
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
        if let Some(milestone_index) = self.gaps.next() {
            log::info!("Requesting unsynced milestone {}.", milestone_index);
            let milestone = self
                .inx_client
                .read_milestone(inx::proto::MilestoneRequest {
                    milestone_index,
                    milestone_id: None,
                })
                .await
                .map(|r| r.into_inner());
            match MilestoneRecord::try_from(milestone?) {
                Ok(rec) => {
                    self.db.upsert_milestone_record(&rec).await?;
                    let cone_stream = self
                        .inx_client
                        .read_milestone_cone(inx::proto::MilestoneRequest {
                            milestone_index: rec.milestone_index,
                            milestone_id: None,
                        })
                        .await?
                        .into_inner();
                    cx.spawn_child(ConeStream::new(self.db.clone()).with_stream(cone_stream))
                        .await;
                }
                Err(e) => {
                    log::error!("Could not read milestone: {:?}", e);
                }
            }
        } else {
            log::info!("Sync complete");
            cx.shutdown();
        }
        Ok(())
    }
}
