// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod cone_stream;
mod config;
mod error;
mod ledger_update_stream;
mod syncer;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, HandleEvent, Report, Sender},
    types::tangle::MilestoneIndex,
};
pub use config::InxConfig;
pub use error::InxError;
use inx::{
    client::InxClient,
    proto::NoParams,
    tonic::{Channel, Code},
    NodeStatus,
};
pub use ledger_update_stream::LedgerUpdateStream;

use self::syncer::{SyncNext, Syncer};

pub struct InxWorker {
    db: MongoDb,
    config: InxConfig,
}

impl InxWorker {
    /// Creates an [`InxClient`] by connecting to the endpoint specified in `inx_config`.
    pub fn new(db: &MongoDb, inx_config: &InxConfig) -> Self {
        Self {
            db: db.clone(),
            config: inx_config.clone(),
        }
    }

    pub async fn connect(inx_config: &InxConfig) -> Result<InxClient<Channel>, InxError> {
        let url = url::Url::parse(&inx_config.connect_url)?;

        if url.scheme() != "http" {
            return Err(InxError::InvalidAddress(inx_config.connect_url.clone()));
        }

        for i in 0..inx_config.connection_retry_count {
            match InxClient::connect(inx_config.connect_url.clone()).await {
                Ok(inx_client) => return Ok(inx_client),
                Err(_) => {
                    log::warn!(
                        "INX connection failed. Retrying in {}s. {} retries remaining.",
                        inx_config.connection_retry_interval.as_secs(),
                        inx_config.connection_retry_count - i
                    );
                    tokio::time::sleep(inx_config.connection_retry_interval).await;
                }
            }
        }
        Err(InxError::ConnectionError)
    }

    async fn spawn_syncer(
        &self,
        cx: &mut ActorContext<Self>,
        inx_client: &mut InxClient<Channel>,
    ) -> Result<MilestoneIndex, InxError> {
        // Request the node status so we can get the pruning index and latest confirmed milestone
        let node_status = NodeStatus::try_from(inx_client.read_node_status(NoParams {}).await?.into_inner())
            .map_err(InxError::InxTypeConversion)?;

        log::debug!(
            "The node has a pruning index of `{}` and a latest confirmed milestone index of `{}`.",
            node_status.tangle_pruning_index,
            node_status.confirmed_milestone.milestone_info.milestone_index
        );

        let node_config = inx_client.read_node_configuration(NoParams {}).await?.into_inner();

        let network_name = node_config.protocol_parameters.unwrap().network_name;

        log::debug!("Connected to network {}.", network_name);

        if let Some(prev_network_name) = self.db.get_network_name().await? {
            if prev_network_name != network_name {
                return Err(InxError::NetworkChanged(prev_network_name, network_name));
            }
        } else {
            log::info!("Linking database {} to network {}.", self.db.name(), network_name);
            self.db.set_network_name(network_name).await?;
        }

        let first_ms = node_status.tangle_pruning_index + 1;
        let latest_ms = node_status.confirmed_milestone.milestone_info.milestone_index;
        let sync_data = self
            .db
            .get_sync_data(self.config.sync_start_milestone.0.max(first_ms).into()..=latest_ms.into())
            .await?
            .gaps;
        if !sync_data.is_empty() {
            let syncer = cx
                .spawn_child(Syncer::new(sync_data, self.db.clone(), inx_client.clone()))
                .await;
            syncer.send(SyncNext)?;
        } else {
            cx.abort().await;
        }
        Ok(latest_ms.into())
    }
}

#[async_trait]
impl Actor for InxWorker {
    type State = InxClient<Channel>;
    type Error = InxError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", &self.config.connect_url);
        let mut inx_client = Self::connect(&self.config).await?;
        log::info!("Connected to INX.");

        let latest_ms = self.spawn_syncer(cx, &mut inx_client).await?;

        cx.spawn_child(LedgerUpdateStream::new(
            self.db.clone(),
            inx_client.clone(),
            latest_ms + 1..=u32::MAX.into(),
        ))
        .await;
        Ok(inx_client)
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Inx Worker".into()
    }
}

#[async_trait]
impl HandleEvent<Report<LedgerUpdateStream>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<LedgerUpdateStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => (),
            Report::Error(e) => match e.error {
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

#[async_trait]
impl HandleEvent<Report<Syncer>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Syncer>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => (),
            Report::Error(report) => match report.error {
                ActorError::Result(e) => match &e {
                    InxError::Read(s) => match s.code() {
                        Code::InvalidArgument => {
                            self.spawn_syncer(cx, inx_client).await?;
                        }
                        _ => Err(e)?,
                    },
                    _ => Err(e)?,
                },
                ActorError::Panic | ActorError::Aborted => {
                    cx.abort().await;
                }
            },
        }
        Ok(())
    }
}
