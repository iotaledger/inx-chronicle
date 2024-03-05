// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod config;
mod error;
#[cfg(feature = "influx")]
mod influx;

use std::time::Duration;

use chronicle::{
    db::{
        mongodb::collections::{
            ApplicationStateCollection, BlockCollection, CommittedSlotCollection, LedgerUpdateCollection,
            OutputCollection, ParentsCollection,
        },
        MongoDb,
    },
    inx::{Inx, InxError},
    model::ledger::{LedgerOutput, LedgerSpent},
    tangle::{Slot, Tangle},
};
use eyre::{bail, Result};
use futures::{StreamExt, TryStreamExt};
use iota_sdk::types::block::{protocol::ProtocolParameters, slot::SlotIndex};
use tokio::{task::JoinSet, try_join};
use tracing::{debug, info, instrument, trace_span, Instrument};

pub use self::{config::InxConfig, error::InxWorkerError};

/// Batch size for insert operations.
pub const INSERT_BATCH_SIZE: usize = 1000;

pub struct InxWorker {
    db: MongoDb,
    config: InxConfig,
    #[cfg(feature = "influx")]
    influx_db: Option<chronicle::db::influxdb::InfluxDb>,
}

impl InxWorker {
    /// Creates an [`Inx`] client by connecting to the endpoint specified in `inx_config`.
    pub fn new(db: MongoDb, inx_config: InxConfig) -> Self {
        Self {
            db,
            config: inx_config,
            #[cfg(feature = "influx")]
            influx_db: None,
        }
    }

    #[cfg(feature = "influx")]
    pub fn set_influx_db(&mut self, influx_db: &chronicle::db::influxdb::InfluxDb) {
        self.influx_db.replace(influx_db.clone());
    }

    async fn connect(&self) -> Result<Inx> {
        let url = url::Url::parse(&self.config.url)?;

        if url.scheme() != "http" {
            bail!(InxWorkerError::InvalidAddress(self.config.url.clone()));
        }

        Ok(Inx::connect(&self.config.url).await?)
    }

    pub async fn run(&mut self) -> Result<()> {
        let (start_index, inx, protocol_params) = self.init().await?;

        let tangle = Tangle::from(inx);

        let mut stream = tangle.slot_stream(start_index..).await?;

        #[cfg(feature = "analytics")]
        let mut analytics_info = influx::analytics::AnalyticsInfo::init(&self.db, self.influx_db.as_ref()).await?;

        debug!("Started listening to ledger updates via INX.");

        while let Some(slot) = stream.try_next().await? {
            self.handle_ledger_update(
                slot,
                &protocol_params,
                #[cfg(feature = "analytics")]
                analytics_info.as_mut(),
            )
            .await?;
        }

        tracing::debug!("INX stream closed unexpectedly.");

        Ok(())
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn init(&mut self) -> Result<(SlotIndex, Inx, ProtocolParameters)> {
        info!("Connecting to INX at bind address `{}`.", &self.config.url);
        let mut inx = self.connect().await?;
        info!("Connected to INX.");

        // Request the node status so we can get the pruning index and latest confirmed slot
        let node_status = loop {
            match inx.get_node_status().await {
                Ok(node_status) => break node_status,
                Err(InxError::MissingField(_)) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(e) => return Err(e.into()),
            };
        };

        debug!(
            "The node has a pruning epoch index of `{}` and a latest confirmed slot index of `{}`.",
            node_status.pruning_epoch,
            node_status.latest_commitment.commitment_id.slot_index()
        );

        let mut node_configuration = inx.get_node_configuration().await?;

        debug!(
            "Connected to network `{}` with base token `{}[{}]`.",
            node_configuration.latest_parameters().network_name(),
            node_configuration.base_token.name,
            node_configuration.base_token.ticker_symbol
        );

        let pruning_slot = node_configuration
            .latest_parameters()
            .first_slot_of(node_status.pruning_epoch);

        // Check if there is an unfixable gap in our node data.
        let mut start_index = if let Some(latest_committed_slot) = self
            .db
            .collection::<CommittedSlotCollection>()
            .get_latest_committed_slot()
            .await?
        {
            if pruning_slot > latest_committed_slot.slot_index {
                bail!(InxWorkerError::SyncSlotGap {
                    start: latest_committed_slot.slot_index + 1,
                    end: pruning_slot,
                });
            } else if node_status.last_accepted_block_slot < latest_committed_slot.slot_index {
                bail!(InxWorkerError::SyncSlotIndexMismatch {
                    node: node_status.last_accepted_block_slot,
                    db: latest_committed_slot.slot_index,
                });
            } else {
                latest_committed_slot.slot_index + 1
            }
        } else {
            self.config.sync_start_slot.max(pruning_slot)
        };
        // Skip the genesis slot
        if start_index == node_configuration.latest_parameters().genesis_slot() {
            start_index += 1;
        }

        if let Some(db_node_config) = self
            .db
            .collection::<ApplicationStateCollection>()
            .get_node_config()
            .await?
        {
            #[allow(clippy::collapsible_if)]
            if db_node_config != node_configuration {
                if db_node_config.latest_parameters().network_name()
                    != node_configuration.latest_parameters().network_name()
                {
                    bail!(InxWorkerError::NetworkChanged {
                        old: db_node_config.latest_parameters().network_name().to_owned(),
                        new: node_configuration.latest_parameters().network_name().to_owned(),
                    });
                }
                // TODO: Maybe we need to do some additional checking?
            }
        } else {
            self.db.clear().await?;

            // let latest_version = LatestMigration::version();
            // info!("Setting migration version to {}", latest_version);
            // self.db
            //     .collection::<ApplicationStateCollection>()
            //     .set_last_migration(latest_version)
            //     .await?;
            info!("Reading unspent outputs.");
            let unspent_output_stream = inx
                .get_unspent_outputs()
                .instrument(trace_span!("inx_read_unspent_outputs"))
                .await?;

            let mut starting_index = None;
            let protocol_parameters = node_configuration.latest_parameters();

            let mut count = 0;
            let mut tasks = unspent_output_stream
                .inspect_ok(|_| count += 1)
                .map(|msg| {
                    let msg = msg?;
                    let ledger_index = &msg.latest_commitment_id.slot_index();
                    if let Some(index) = starting_index.as_ref() {
                        if index != ledger_index {
                            bail!(InxWorkerError::InvalidUnspentOutputIndex {
                                found: *ledger_index,
                                expected: *index,
                            })
                        }
                    } else {
                        starting_index = Some(*ledger_index);
                    }
                    Ok(msg)
                })
                .map(|res| Ok(res?.output))
                .try_chunks(INSERT_BATCH_SIZE)
                // We only care if we had an error, so discard the other data
                .map_err(|e| e.1)
                // Convert batches to tasks
                .try_fold(JoinSet::new(), |mut tasks, batch| async {
                    let db = self.db.clone();
                    let protocol_parameters = protocol_parameters.clone();
                    tasks.spawn(async move { insert_unspent_outputs(&db, &batch, &protocol_parameters).await });
                    Result::<_>::Ok(tasks)
                })
                .await?;

            while let Some(res) = tasks.join_next().await {
                res??;
            }

            info!("Inserted {} unspent outputs.", count);

            let starting_index = starting_index.unwrap_or(SlotIndex(0));

            // Get the timestamp for the starting index
            let slot_timestamp = starting_index.to_timestamp(
                protocol_parameters.genesis_unix_timestamp(),
                protocol_parameters.slot_duration_in_seconds(),
            );

            info!(
                "Setting starting index to {} with timestamp {}",
                starting_index,
                time::OffsetDateTime::from_unix_timestamp(slot_timestamp as _)?
                    .format(&time::format_description::well_known::Rfc3339)?
            );

            self.db
                .collection::<ApplicationStateCollection>()
                .set_starting_index(starting_index)
                .await?;

            info!(
                "Linking database `{}` to network `{}`.",
                self.db.name(),
                protocol_parameters.network_name()
            );
        }

        debug!("Updating node configuration.");
        self.db
            .collection::<ApplicationStateCollection>()
            .set_node_config(&node_configuration)
            .await?;

        Ok((
            start_index,
            inx,
            node_configuration.protocol_parameters.pop().unwrap().parameters,
        ))
    }

    #[instrument(skip_all, fields(slot_index, created, consumed), err, level = "debug")]
    async fn handle_ledger_update<'a>(
        &mut self,
        slot: Slot<'a, Inx>,
        protocol_parameters: &ProtocolParameters,
        #[cfg(feature = "analytics")] analytics_info: Option<&mut influx::analytics::AnalyticsInfo>,
    ) -> Result<()> {
        #[cfg(feature = "metrics")]
        let start_time = std::time::Instant::now();

        let mut tasks = JoinSet::new();

        for batch in slot.ledger_updates().created_outputs().chunks(INSERT_BATCH_SIZE) {
            let db = self.db.clone();
            let batch = batch.to_vec();
            let protocol_parameters = protocol_parameters.clone();
            tasks.spawn(async move { insert_unspent_outputs(&db, &batch, &protocol_parameters).await });
        }

        for batch in slot.ledger_updates().consumed_outputs().chunks(INSERT_BATCH_SIZE) {
            let db = self.db.clone();
            let batch = batch.to_vec();
            let protocol_parameters = protocol_parameters.clone();
            tasks.spawn(async move { update_spent_outputs(&db, &batch, &protocol_parameters).await });
        }

        while let Some(res) = tasks.join_next().await {
            res??;
        }

        // Record the result as part of the current span.
        tracing::Span::current().record("slot_index", slot.index().0);
        tracing::Span::current().record("created", slot.ledger_updates().created_outputs().len());
        tracing::Span::current().record("consumed", slot.ledger_updates().consumed_outputs().len());

        self.handle_accepted_blocks(&slot).await?;

        #[cfg(feature = "influx")]
        self.update_influx(
            &slot,
            protocol_parameters,
            #[cfg(feature = "analytics")]
            analytics_info,
            #[cfg(feature = "metrics")]
            start_time,
        )
        .await?;

        // This acts as a checkpoint for the syncing and has to be done last, after everything else completed.
        self.db
            .collection::<CommittedSlotCollection>()
            .upsert_committed_slot(slot.index(), slot.commitment_id(), slot.commitment().clone())
            .await?;

        Ok(())
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn handle_accepted_blocks<'a>(&mut self, slot: &Slot<'a, Inx>) -> Result<()> {
        let blocks_stream = slot.accepted_block_stream().await?;

        let mut tasks = blocks_stream
            .try_chunks(INSERT_BATCH_SIZE)
            .map_err(|e| e.1)
            .try_fold(JoinSet::new(), |mut tasks, batch| async {
                let db = self.db.clone();
                tasks.spawn(async move {
                    db.collection::<ParentsCollection>().insert_blocks(&batch).await?;
                    db.collection::<BlockCollection>()
                        .insert_blocks_with_metadata(batch)
                        .await?;
                    Result::<_>::Ok(())
                });
                Ok(tasks)
            })
            .await?;

        while let Some(res) = tasks.join_next().await {
            res??;
        }

        Ok(())
    }
}

#[instrument(skip_all, err, fields(num = outputs.len()), level = "trace")]
async fn insert_unspent_outputs(db: &MongoDb, outputs: &[LedgerOutput], params: &ProtocolParameters) -> Result<()> {
    let output_collection = db.collection::<OutputCollection>();
    let ledger_collection = db.collection::<LedgerUpdateCollection>();
    try_join! {
        async {
            output_collection.insert_unspent_outputs(outputs, params).await?;
            Result::<_>::Ok(())
        },
        async {
            ledger_collection.insert_unspent_ledger_updates(outputs, params).await?;
            Ok(())
        }
    }?;
    Ok(())
}

#[instrument(skip_all, err, fields(num = outputs.len()), level = "trace")]
async fn update_spent_outputs(db: &MongoDb, outputs: &[LedgerSpent], params: &ProtocolParameters) -> Result<()> {
    let output_collection = db.collection::<OutputCollection>();
    let ledger_collection = db.collection::<LedgerUpdateCollection>();
    try_join! {
        async {
            output_collection.update_spent_outputs(outputs, params).await?;
            Ok(())
        },
        async {
            ledger_collection.insert_spent_ledger_updates(outputs, params).await?;
            Ok(())
        }
    }
    .and(Ok(()))
}
