// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;

use async_trait::async_trait;
use bee_inx::client::Inx;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent},
    types::{ledger::MilestoneIndexTimestamp, tangle::ProtocolParameters},
};
pub use config::InxConfig;
pub use error::InxError;
use futures::{StreamExt, TryStreamExt};
use tokio::time::Instant;

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
}

#[async_trait]
impl Actor for InxWorker {
    type State = Inx;
    type Error = InxError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", &self.config.connect_url);
        let mut inx = Self::connect(&self.config).await?;
        log::info!("Connected to INX.");

        // Request the node status so we can get the pruning index and latest confirmed milestone
        let node_status = inx.read_node_status().await?;

        log::debug!(
            "The node has a pruning index of `{}` and a latest confirmed milestone index of `{}`.",
            node_status.tangle_pruning_index,
            node_status.confirmed_milestone.milestone_info.milestone_index,
        );

        // Check if there is an unfixable gap in our node data.
        let start_index = if let Some(MilestoneIndexTimestamp {
            milestone_index: latest_milestone,
            ..
        }) = self.db.get_latest_milestone().await?
        {
            if node_status.tangle_pruning_index > latest_milestone.0 {
                return Err(InxError::MilestoneGap {
                    start: latest_milestone + 1,
                    end: node_status.tangle_pruning_index.into(),
                });
            }
            latest_milestone + 1
        } else {
            self.config
                .sync_start_milestone
                .max((node_status.tangle_pruning_index + 1).into())
        };

        let protocol_parameters: ProtocolParameters = inx
            .read_protocol_parameters(start_index.0.into())
            .await?
            .inner()?
            .into();

        log::debug!("Connected to network `{}`.", protocol_parameters.network_name);

        let mut session = self.db.start_transaction(None).await?;

        if let Some(latest) = self.db.get_latest_protocol_parameters().await? {
            if latest.parameters.network_name != protocol_parameters.network_name {
                return Err(InxError::NetworkChanged(
                    latest.parameters.network_name,
                    protocol_parameters.network_name,
                ));
            }
            if latest.parameters != protocol_parameters {
                self.db
                    .insert_protocol_parameters(&mut session, start_index, protocol_parameters)
                    .await?;
            }
        } else {
            log::info!(
                "Linking database `{}` to network `{}`.",
                self.db.name(),
                protocol_parameters.network_name
            );

            self.db
                .insert_protocol_parameters(&mut session, start_index, protocol_parameters)
                .await?;

            log::info!("Reading unspent outputs.");
            let mut unspent_output_stream = inx.read_unspent_outputs().await?;

            let mut updates = Vec::new();
            while let Some(bee_inx::UnspentOutput { output, .. }) = unspent_output_stream.try_next().await? {
                log::trace!("Received unspent output: {}", output.block_id);
                updates.push(output.try_into()?);
            }
            log::info!("Inserting {} unspent outputs.", updates.len());

            // TODO: Use tracing here.
            let start_time = Instant::now();
            self.db.insert_ledger_updates(&mut session, updates).await?;
            let duration = start_time.elapsed();
            log::info!(
                "Inserting unspent outputs took {}.",
                humantime::Duration::from(duration)
            );
        }

        session.commit_transaction().await?;

        let ledger_update_stream = inx.listen_to_ledger_updates((start_index.0..).into()).await?;

        cx.add_stream(ledger_update_stream);

        Ok(inx)
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Inx Worker".into()
    }
}

#[async_trait]
impl HandleEvent<Result<bee_inx::LedgerUpdate, bee_inx::Error>> for InxWorker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        ledger_update_result: Result<bee_inx::LedgerUpdate, bee_inx::Error>,
        inx: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received ledger update event {:#?}", ledger_update_result);

        // TODO: Use tracing here.
        let start_time = Instant::now();

        let ledger_update = ledger_update_result?;

        let output_updates = Vec::from(ledger_update.created)
            .into_iter()
            .map(TryInto::try_into)
            .chain(Vec::from(ledger_update.consumed).into_iter().map(TryInto::try_into))
            .collect::<Result<Vec<_>, _>>()?;

        let mut session = self.db.start_transaction(None).await?;

        self.db
            .insert_ledger_updates(&mut session, output_updates.into_iter())
            .await?;

        let milestone = inx.read_milestone(ledger_update.milestone_index.into()).await?;
        let parameters: ProtocolParameters = inx
            .read_protocol_parameters(ledger_update.milestone_index.into())
            .await?
            .inner()?
            .into();

        self.db
            .update_latest_protocol_parameters(&mut session, ledger_update.milestone_index.into(), parameters)
            .await?;

        log::trace!("Received milestone: `{:?}`", milestone);

        let milestone_index = milestone.milestone_info.milestone_index.into();
        let milestone_timestamp = milestone.milestone_info.milestone_timestamp.into();
        let milestone_id = milestone
            .milestone_info
            .milestone_id
            .ok_or(Self::Error::MissingMilestoneInfo(milestone_index))?
            .into();
        let payload = Into::into(
            &milestone
                .milestone
                .ok_or(Self::Error::MissingMilestoneInfo(milestone_index))?,
        );

        let cone_stream = inx.read_milestone_cone(milestone_index.0.into()).await?;

        let blocks_with_metadata = cone_stream
            .map(|res| {
                let bee_inx::BlockWithMetadata { block, metadata } = res?;
                Result::<_, Self::Error>::Ok((block.clone().inner()?.into(), block.data(), metadata.into()))
            })
            .try_collect::<Vec<_>>()
            .await?;

        self.db
            .insert_blocks_with_metadata(&mut session, blocks_with_metadata)
            .await?;

        self.db
            .insert_milestone(
                &mut session,
                milestone_id,
                milestone_index,
                milestone_timestamp,
                payload,
            )
            .await?;

        session.commit_transaction().await?;

        let duration = start_time.elapsed();
        log::debug!(
            "Milestone `{}` synced in {}.",
            milestone_index,
            humantime::Duration::from(duration)
        );

        Ok(())
    }
}
