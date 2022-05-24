// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{model::stardust::milestone::MilestoneRecord, MongoDb},
    runtime::{Actor, ActorContext, ActorError, ConfigureActor, HandleEvent, Report},
};
use inx::{
    client::InxClient,
    tonic::{Channel, Status},
};

use super::{cone_stream::ConeStream, InxError};

#[derive(Debug)]
pub struct MilestoneStream {
    db: MongoDb,
    inx_client: InxClient<Channel>,
}

impl MilestoneStream {
    pub fn new(db: MongoDb, inx_client: InxClient<Channel>) -> Self {
        Self { db, inx_client }
    }
}

#[async_trait]
impl Actor for MilestoneStream {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<ConeStream>> for MilestoneStream {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<ConeStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(report) => {
                self.db.upsert_sync_record(report.actor.milestone_index).await?;
            }
            Report::Error(report) => match report.error {
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

#[async_trait]
impl HandleEvent<Result<inx::proto::Milestone, Status>> for MilestoneStream {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        milestone: Result<inx::proto::Milestone, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received Stardust Milestone Event");
        match MilestoneRecord::try_from(milestone?) {
            Ok(rec) => {
                self.db.upsert_milestone_record(&rec).await?;
                let cone_stream = self
                    .inx_client
                    .read_milestone_cone(inx::proto::MilestoneRequest {
                        milestone_index: *rec.milestone_index,
                        milestone_id: None,
                    })
                    .await?
                    .into_inner();
                cx.spawn_child(ConeStream::new(rec.milestone_index, self.db.clone()).with_stream(cone_stream))
                    .await;
            }
            Err(e) => {
                log::error!("Could not read milestone: {:?}", e);
            }
        }
        Ok(())
    }
}
