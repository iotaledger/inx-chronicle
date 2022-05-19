// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{model::stardust::milestone::MilestoneRecord, MongoDb},
    runtime::{Actor, ActorContext, ActorError, ConfigureActor, HandleEvent, Report},
};
use inx::{
    client::InxClient,
    proto::NoParams,
    tonic::{Channel, Status},
    NodeStatus,
};

use super::{
    cone_stream::ConeStream,
    config::SyncKind,
    syncer::{SyncNext, Syncer},
    InxConfig, InxError,
};

#[derive(Debug)]
pub struct MilestoneStream {
    db: MongoDb,
    inx_client: InxClient<Channel>,
    config: InxConfig,
    latest_ms: u32,
}

impl MilestoneStream {
    pub fn new(db: MongoDb, inx_client: InxClient<Channel>, config: InxConfig, latest_ms: u32) -> Self {
        Self {
            db,
            inx_client,
            config,
            latest_ms,
        }
    }
}

#[async_trait]
impl Actor for MilestoneStream {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        // Request the node status so we can get the pruning index
        let node_status = NodeStatus::try_from(self.inx_client.read_node_status(NoParams {}).await?.into_inner())
            .map_err(InxError::InxTypeConversion)?;
        let configured_start = match self.config.sync_kind {
            SyncKind::Max(ms) => self.latest_ms - ms,
            SyncKind::From(ms) => ms,
        };
        let sync_data = self
            .db
            .get_sync_data(configured_start.max(node_status.pruning_index), self.latest_ms)
            .await?
            .gaps;
        if !sync_data.is_empty() {
            let syncer = cx
                .spawn_child(Syncer::new(sync_data, self.db.clone(), self.inx_client.clone()))
                .await;
            for _ in 0..self.config.max_parallel_requests {
                syncer.send(SyncNext)?;
            }
        } else {
            cx.shutdown();
        }
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
            Report::Success(_) => (),
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
impl HandleEvent<Report<Syncer>> for MilestoneStream {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Syncer>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => (),
            Report::Error(report) => match report.error {
                ActorError::Result(e) => {
                    Err(e)?;
                }
                ActorError::Panic | ActorError::Aborted => {
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
        Ok(())
    }
}
