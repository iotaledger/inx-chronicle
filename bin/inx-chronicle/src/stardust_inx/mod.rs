// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;
mod ledger_update_stream;
mod syncer;

use async_trait::async_trait;
use bee_inx::client::Inx;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, HandleEvent, Report, Sender},
    types::tangle::{MilestoneIndex, ProtocolInfo, ProtocolParameters},
};
pub use config::InxConfig;
pub use error::InxError;
use futures::TryStreamExt;
pub use ledger_update_stream::LedgerUpdateStream;

use self::syncer::{SyncNext, Syncer};

pub struct InxWorker {
    db: MongoDb,
    config: InxConfig,
}

impl InxWorker {
    /// Creates an [`Inx`] client by connecting to the endpoint specified in `inx_config`.
    pub fn new(db: &MongoDb, inx_config: &InxConfig) -> Self {
        Self {
            db: db.clone(),
            config: inx_config.clone(),
        }
    }

    pub async fn connect(inx_config: &InxConfig) -> Result<Inx, InxError> {
        let url = url::Url::parse(&inx_config.connect_url)?;

        if url.scheme() != "http" {
            return Err(InxError::InvalidAddress(inx_config.connect_url.clone()));
        }

        for i in 0..inx_config.connection_retry_count {
            match Inx::connect(inx_config.connect_url.clone()).await {
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

    async fn spawn_syncer(&self, cx: &mut ActorContext<Self>, inx: &mut Inx) -> Result<MilestoneIndex, InxError> {
        // Request the node status so we can get the pruning index and latest confirmed milestone
        let node_status = inx.read_node_status().await?;

        let latest_milestone_index = node_status.confirmed_milestone.milestone_info.milestone_index;

        log::debug!(
            "The node has a pruning index of `{}` and a latest confirmed milestone index of `{latest_milestone_index}`.",
            node_status.tangle_pruning_index,
        );

        let protocol_parameters: ProtocolParameters = inx
            .read_protocol_parameters(latest_milestone_index.into())
            .await?
            .inner()?
            .into();

        log::debug!("Connected to network `{}`.", protocol_parameters.network_name);

        if let Some(db_protocol) = self.db.get_protocol_parameters().await? {
            if db_protocol.parameters.network_name != protocol_parameters.network_name {
                return Err(InxError::NetworkChanged(
                    db_protocol.parameters.network_name,
                    protocol_parameters.network_name,
                ));
            }
        } else {
            log::info!(
                "Linking database `{}` to network `{}`.",
                self.db.name(),
                protocol_parameters.network_name
            );

            let protocol_info = ProtocolInfo {
                parameters: protocol_parameters,
                tangle_index: latest_milestone_index.into(),
            };

            self.db.set_protocol_parameters(protocol_info).await?;
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
                .spawn_child(Syncer::new(sync_data, self.db.clone(), inx.clone()))
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
    type State = Inx;
    type Error = InxError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", &self.config.connect_url);
        let mut inx = Self::connect(&self.config).await?;
        log::info!("Connected to INX.");

        log::info!("Reading unspent outputs.");
        let mut unspent_output_stream = inx.read_unspent_outputs().await?;

        let mut updates = Vec::new();
        while let Some(bee_inx::UnspentOutput { output, .. }) = unspent_output_stream.try_next().await? {
            log::trace!("Received unspent output: {}", output.block_id);
            updates.push(output.try_into()?);
        }
        log::info!("Inserting {} unspent outputs.", updates.len());
        self.db.insert_ledger_updates(updates).await?;

        let latest_ms = self.spawn_syncer(cx, &mut inx).await?;

        cx.spawn_child(LedgerUpdateStream::new(
            self.db.clone(),
            inx.clone(),
            latest_ms + 1..=u32::MAX.into(),
        ))
        .await;

        Ok(inx)
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
        inx: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => (),
            Report::Error(report) => match report.error {
                ActorError::Result(e) => match &e {
                    InxError::BeeInx(bee_inx::Error::StatusCode(s)) => match s.code() {
                        tonic::Code::InvalidArgument => {
                            self.spawn_syncer(cx, inx).await?;
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
