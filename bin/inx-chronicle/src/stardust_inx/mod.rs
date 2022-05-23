// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod cone_stream;
mod config;
mod error;
mod milestone_stream;
mod syncer;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, ConfigureActor, HandleEvent, Report},
};
pub use config::InxConfig;
pub use error::InxError;
use inx::{client::InxClient, proto::NoParams, tonic::Channel, NodeStatus};
pub use milestone_stream::MilestoneStream;

use self::{
    config::SyncKind,
    syncer::{SyncNext, Syncer},
};

pub struct Inx {
    db: MongoDb,
    config: InxConfig,
}

impl Inx {
    /// Creates an [`InxClient`] by connecting to the endpoint specified in `inx_config`.
    pub fn new(db: MongoDb, inx_config: InxConfig) -> Self {
        Self { db, config: inx_config }
    }

    pub async fn connect(inx_config: &InxConfig) -> Result<InxClient<Channel>, InxError> {
        let url = url::Url::parse(&inx_config.connect_url)?;

        if url.scheme() != "http" {
            return Err(InxError::InvalidAddress(inx_config.connect_url.clone()));
        }

        InxClient::connect(inx_config.connect_url.clone())
            .await
            .map_err(InxError::ConnectionError)
    }
}

#[async_trait]
impl Actor for Inx {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", &self.config.connect_url);
        let mut inx_client = Self::connect(&self.config).await?;
        log::info!("Connected to INX.");

        // Request the node status so we can get the pruning index and latest confirmed milestone
        let node_status = NodeStatus::try_from(inx_client.read_node_status(NoParams {}).await?.into_inner())
            .map_err(InxError::InxTypeConversion)?;
        let latest_ms = node_status.confirmed_milestone.milestone_info.milestone_index;
        let configured_start = match self.config.sync_kind {
            SyncKind::Max(ms) => latest_ms - ms,
            SyncKind::From(ms) => ms,
        };
        let sync_data = self
            .db
            .get_sync_data(configured_start.max(node_status.tangle_pruning_index), latest_ms)
            .await?
            .gaps;
        if !sync_data.is_empty() {
            let syncer = cx
                .spawn_child(Syncer::new(sync_data, self.db.clone(), inx_client.clone()))
                .await;
            syncer.send(SyncNext)?;
        } else {
            cx.shutdown();
        }

        let milestone_stream = inx_client
            .listen_to_confirmed_milestones(inx::proto::MilestoneRangeRequest::from(
                inx::MilestoneRangeRequest::FromMilestoneIndex(
                    node_status.confirmed_milestone.milestone_info.milestone_index + 1,
                ),
            ))
            .await?
            .into_inner();
        cx.spawn_child(MilestoneStream::new(self.db.clone(), inx_client).with_stream(milestone_stream))
            .await;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<MilestoneStream>> for Inx {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MilestoneStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
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

#[async_trait]
impl HandleEvent<Report<Syncer>> for Inx {
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
