// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod config;
mod error;
#[cfg(feature = "influx")]
mod influx;

use std::time::Duration;

use chronicle::{
    db::{
        mongodb::collections::{
            ApplicationStateCollection, BlockCollection, ConfigurationUpdateCollection, LedgerUpdateCollection,
            MilestoneCollection, OutputCollection, ProtocolUpdateCollection, TreasuryCollection,
        },
        MongoDb,
    },
    inx::{Inx, InxError},
    model::{
        ledger::{LedgerOutput, LedgerSpent},
        metadata::LedgerInclusionState,
        payload::Payload,
        tangle::{MilestoneIndex, MilestoneIndexTimestamp},
    },
    tangle::{Milestone, Tangle},
};
use eyre::{bail, Result};
use futures::{StreamExt, TryStreamExt};
use tokio::{task::JoinSet, try_join};
use tracing::{debug, info, instrument, trace_span, Instrument};

pub use self::{config::InxConfig, error::InxWorkerError};
use crate::migrations::{LatestMigration, Migration};

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
    pub fn new(db: &MongoDb, inx_config: &InxConfig) -> Self {
        Self {
            db: db.clone(),
            config: inx_config.clone(),
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

        Ok(Inx::connect(self.config.url.clone()).await?)
    }

    pub async fn run(&mut self) -> Result<()> {
        let (start_index, inx) = self.init().await?;

        let tangle = Tangle::from(inx);

        let mut stream = tangle.milestone_stream(start_index..).await?;

        #[cfg(feature = "analytics")]
        let mut analytics_info = influx::analytics::AnalyticsInfo::init(&self.db, self.influx_db.as_ref()).await?;

        debug!("Started listening to ledger updates via INX.");

        while let Some(milestone) = stream.try_next().await? {
            self.handle_ledger_update(
                milestone,
                #[cfg(feature = "analytics")]
                analytics_info.as_mut(),
            )
            .await?;
        }

        tracing::debug!("INX stream closed unexpectedly.");

        Ok(())
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn init(&mut self) -> Result<(MilestoneIndex, Inx)> {
        info!("Connecting to INX at bind address `{}`.", &self.config.url);
        let mut inx = self.connect().await?;
        info!("Connected to INX.");

        // Request the node status so we can get the pruning index and latest confirmed milestone
        let node_status = loop {
            match inx.read_node_status().await {
                Ok(node_status) => break node_status,
                Err(InxError::MissingField(_)) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(e) => return Err(e.into()),
            };
        };

        debug!(
            "The node has a pruning index of `{}` and a latest confirmed milestone index of `{}`.",
            node_status.tangle_pruning_index, node_status.confirmed_milestone.milestone_info.milestone_index,
        );

        // Check if there is an unfixable gap in our node data.
        let start_index = if let Some(MilestoneIndexTimestamp {
            milestone_index: latest_milestone,
            ..
        }) = self
            .db
            .collection::<MilestoneCollection>()
            .get_newest_milestone()
            .await?
        {
            if node_status.tangle_pruning_index.0 > latest_milestone.0 {
                bail!(InxWorkerError::SyncMilestoneGap {
                    start: latest_milestone + 1,
                    end: node_status.tangle_pruning_index,
                });
            } else if node_status.confirmed_milestone.milestone_info.milestone_index.0 < latest_milestone.0 {
                bail!(InxWorkerError::SyncMilestoneIndexMismatch {
                    node: node_status.confirmed_milestone.milestone_info.milestone_index,
                    db: latest_milestone,
                });
            } else {
                latest_milestone + 1
            }
        } else {
            self.config
                .sync_start_milestone
                .max(node_status.tangle_pruning_index + 1)
        };

        let protocol_parameters = inx
            .read_protocol_parameters(start_index.0.into())
            .await?
            .params
            .inner_unverified()?;

        let node_configuration = inx.read_node_configuration().await?;

        debug!(
            "Connected to network `{}` with base token `{}[{}]`.",
            protocol_parameters.network_name(),
            node_configuration.base_token.name,
            node_configuration.base_token.ticker_symbol
        );

        self.db
            .collection::<ConfigurationUpdateCollection>()
            .upsert_node_configuration(node_status.ledger_index, node_configuration.into())
            .await?;

        if let Some(latest) = self
            .db
            .collection::<ProtocolUpdateCollection>()
            .get_latest_protocol_parameters()
            .await?
        {
            let protocol_parameters = chronicle::model::ProtocolParameters::from(protocol_parameters);
            if latest.parameters.network_name != protocol_parameters.network_name {
                bail!(InxWorkerError::NetworkChanged {
                    old: latest.parameters.network_name,
                    new: protocol_parameters.network_name,
                });
            }
            debug!("Found matching network in the database.");
            if latest.parameters != protocol_parameters {
                debug!("Updating protocol parameters.");
                self.db
                    .collection::<ProtocolUpdateCollection>()
                    .upsert_protocol_parameters(start_index, protocol_parameters)
                    .await?;
            }
        } else {
            self.db.clear().await?;

            let latest_version = LatestMigration::version();
            info!("Setting migration version to {}", latest_version);
            self.db
                .collection::<ApplicationStateCollection>()
                .set_last_migration(latest_version)
                .await?;
            info!("Reading unspent outputs.");
            let unspent_output_stream = inx
                .read_unspent_outputs()
                .instrument(trace_span!("inx_read_unspent_outputs"))
                .await?;

            let mut starting_index = None;

            let mut count = 0;
            let mut tasks = unspent_output_stream
                .inspect_ok(|_| count += 1)
                .map(|msg| {
                    let msg = msg?;
                    let ledger_index = &msg.ledger_index;
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
                    tasks.spawn(async move { insert_unspent_outputs(&db, &batch).await });
                    Result::<_>::Ok(tasks)
                })
                .await?;

            while let Some(res) = tasks.join_next().await {
                res??;
            }

            info!("Inserted {} unspent outputs.", count);

            let starting_index = starting_index.unwrap_or_default();

            // Get the timestamp for the starting index
            let milestone_timestamp = inx
                .read_milestone(starting_index.into())
                .await?
                .milestone_info
                .milestone_timestamp
                .into();

            info!(
                "Setting starting index to {} with timestamp {}",
                starting_index,
                time::OffsetDateTime::try_from(milestone_timestamp)?
                    .format(&time::format_description::well_known::Rfc3339)?
            );

            let starting_index = starting_index.with_timestamp(milestone_timestamp);

            self.db
                .collection::<ApplicationStateCollection>()
                .set_starting_index(starting_index)
                .await?;

            info!(
                "Linking database `{}` to network `{}`.",
                self.db.name(),
                protocol_parameters.network_name()
            );

            self.db
                .collection::<ProtocolUpdateCollection>()
                .upsert_protocol_parameters(start_index, protocol_parameters.into())
                .await?;
        }

        Ok((start_index, inx))
    }

    #[instrument(skip_all, fields(milestone_index, created, consumed), err, level = "debug")]
    async fn handle_ledger_update<'a>(
        &mut self,
        milestone: Milestone<'a, Inx>,
        #[cfg(feature = "analytics")] analytics_info: Option<&mut influx::analytics::AnalyticsInfo>,
    ) -> Result<()> {
        #[cfg(feature = "metrics")]
        let start_time = std::time::Instant::now();

        let mut tasks = JoinSet::new();

        for batch in milestone.ledger_updates().created_outputs().chunks(INSERT_BATCH_SIZE) {
            let db = self.db.clone();
            let batch = batch.to_vec();
            tasks.spawn(async move { insert_unspent_outputs(&db, &batch).await });
        }

        for batch in milestone.ledger_updates().consumed_outputs().chunks(INSERT_BATCH_SIZE) {
            let db = self.db.clone();
            let batch = batch.to_vec();
            tasks.spawn(async move { update_spent_outputs(&db, &batch).await });
        }

        while let Some(res) = tasks.join_next().await {
            res??;
        }

        // Record the result as part of the current span.
        tracing::Span::current().record("milestone_index", milestone.at.milestone_index.0);
        tracing::Span::current().record("created", milestone.ledger_updates().created_outputs().len());
        tracing::Span::current().record("consumed", milestone.ledger_updates().consumed_outputs().len());

        self.handle_cone_stream(&milestone).await?;
        self.db
            .collection::<ProtocolUpdateCollection>()
            .upsert_protocol_parameters(milestone.at.milestone_index, milestone.protocol_params.clone())
            .await?;
        self.db
            .collection::<ConfigurationUpdateCollection>()
            .upsert_node_configuration(milestone.at.milestone_index, milestone.node_config.clone())
            .await?;

        #[cfg(feature = "influx")]
        self.update_influx(
            &milestone,
            #[cfg(feature = "analytics")]
            analytics_info,
            #[cfg(feature = "metrics")]
            start_time,
        )
        .await?;

        // This acts as a checkpoint for the syncing and has to be done last, after everything else completed.
        self.db
            .collection::<MilestoneCollection>()
            .insert_milestone(
                milestone.milestone_id,
                milestone.at.milestone_index,
                milestone.at.milestone_timestamp,
                milestone.payload.clone(),
            )
            .await?;

        Ok(())
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn handle_cone_stream<'a>(&mut self, milestone: &Milestone<'a, Inx>) -> Result<()> {
        let cone_stream = milestone.cone_stream().await?;

        let mut tasks = cone_stream
            .try_chunks(INSERT_BATCH_SIZE)
            .map_err(|e| e.1)
            .try_fold(JoinSet::new(), |mut tasks, batch| async {
                let db = self.db.clone();
                tasks.spawn(async move {
                    let payloads = batch
                        .iter()
                        .filter_map(|data| {
                            if data.metadata.inclusion_state == LedgerInclusionState::Included {
                                if let Some(Payload::TreasuryTransaction(payload)) = &data.block.payload {
                                    return Some((
                                        data.metadata.referenced_by_milestone_index,
                                        payload.input_milestone_id,
                                        payload.output_amount,
                                    ));
                                }
                            }
                            None
                        })
                        .collect::<Vec<_>>();
                    if !payloads.is_empty() {
                        db.collection::<TreasuryCollection>()
                            .insert_treasury_payloads(payloads)
                            .await?;
                    }
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
async fn insert_unspent_outputs(db: &MongoDb, outputs: &[LedgerOutput]) -> Result<()> {
    let output_collection = db.collection::<OutputCollection>();
    let ledger_collection = db.collection::<LedgerUpdateCollection>();
    try_join! {
        async {
            output_collection.insert_unspent_outputs(outputs).await?;
            Result::<_>::Ok(())
        },
        async {
            ledger_collection.insert_unspent_ledger_updates(outputs).await?;
            Ok(())
        }
    }?;
    Ok(())
}

#[instrument(skip_all, err, fields(num = outputs.len()), level = "trace")]
async fn update_spent_outputs(db: &MongoDb, outputs: &[LedgerSpent]) -> Result<()> {
    let output_collection = db.collection::<OutputCollection>();
    let ledger_collection = db.collection::<LedgerUpdateCollection>();
    try_join! {
        async {
            output_collection.update_spent_outputs(outputs).await?;
            Ok(())
        },
        async {
            ledger_collection.insert_spent_ledger_updates(outputs).await?;
            Ok(())
        }
    }
    .and(Ok(()))
}
