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
use futures::StreamExt;
use inx::{client::InxClient, proto::NoParams, tonic::Channel};
pub use milestone_stream::MilestoneStream;

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
        let mut milestone_stream = inx_client
            .listen_to_confirmed_milestone(NoParams {})
            .await?
            .into_inner();
        let first_ms = milestone_stream.next().await.ok_or(InxError::MilestoneGap)??;
        cx.spawn_child(
            MilestoneStream::new(
                self.db.clone(),
                inx_client,
                self.config.clone(),
                first_ms.milestone_info.unwrap().milestone_index,
            )
            .with_stream(milestone_stream),
        )
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
