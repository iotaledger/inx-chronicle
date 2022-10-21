// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;

use std::{sync::Arc, time::Duration};

use chronicle::{
    db::{
        collections::{
            Analytics, AnalyticsProcessor, BlockCollection, ConfigurationUpdateCollection, LedgerUpdateCollection,
            MilestoneCollection, OutputCollection, ProtocolUpdateCollection, TreasuryCollection,
        },
        InfluxDb, MongoDb,
    },
    inx::{BlockWithMetadataMessage, Inx, InxError, LedgerUpdateMessage, MarkerMessage},
    types::{
        ledger::{BlockMetadata, LedgerInclusionState, LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::{Block, BlockId, Payload},
        tangle::MilestoneIndex,
    },
};
use futures::{StreamExt, TryStreamExt};
use tokio::{sync::Mutex, task::JoinSet, try_join};
use tracing::{debug, info, instrument, trace, trace_span, warn, Instrument};

pub use self::{config::InxConfig, error::InxWorkerError};

/// Batch size for insert operations.
pub const INSERT_BATCH_SIZE: usize = 10000;

pub struct InxWorker {
    db: MongoDb,
    influx_db: InfluxDb,
    analytics: Option<Analytics>,
    config: InxConfig,
}

const METRIC_MILESTONE_INDEX: &str = "milestone_index";
const METRIC_MILESTONE_TIMESTAMP: &str = "milestone_timestamp";
const METRIC_MILESTONE_SYNC_TIME: &str = "milestone_sync_time";

impl InxWorker {
    /// Creates an [`Inx`] client by connecting to the endpoint specified in `inx_config`.
    pub fn new(db: &MongoDb, influx_db: &InfluxDb, inx_config: &InxConfig) -> Self {
        Self {
            db: db.clone(),
            influx_db: influx_db.clone(),
            analytics: None,
            config: inx_config.clone(),
        }
    }

    async fn connect(&self) -> Result<Inx, InxWorkerError> {
        let url = url::Url::parse(&self.config.connect_url)?;

        if url.scheme() != "http" {
            return Err(InxWorkerError::InvalidAddress(self.config.connect_url.clone()));
        }

        for i in 0..self.config.connection_retry_count {
            match Inx::connect(self.config.connect_url.clone()).await {
                Ok(inx_client) => return Ok(inx_client),
                Err(_) => {
                    warn!(
                        "INX connection failed. Retrying in {}s. {} retries remaining.",
                        self.config.connection_retry_interval.as_secs(),
                        self.config.connection_retry_count - i
                    );
                    tokio::time::sleep(self.config.connection_retry_interval).await;
                }
            }
        }
        Err(InxWorkerError::ConnectionError)
    }

    pub async fn run(&mut self) -> Result<(), InxWorkerError> {
        let (start_index, mut inx) = self.init().await?;

        let mut stream = inx.listen_to_ledger_updates((start_index.0..).into()).await?;

        debug!("Started listening to ledger updates via INX.");

        while let Some(ledger_update) = stream.try_next().await? {
            self.handle_ledger_update(&mut inx, ledger_update, &mut stream).await?;
        }

        tracing::debug!("INX stream closed unexpectedly.");

        Ok(())
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn init(&mut self) -> Result<(MilestoneIndex, Inx), InxWorkerError> {
        info!("Connecting to INX at bind address `{}`.", &self.config.connect_url);
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
                return Err(InxWorkerError::SyncMilestoneGap {
                    start: latest_milestone + 1,
                    end: node_status.tangle_pruning_index,
                });
            } else if node_status.confirmed_milestone.milestone_info.milestone_index.0 < latest_milestone.0 {
                return Err(InxWorkerError::SyncMilestoneIndexMismatch {
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
            .update_latest_node_configuration(node_status.ledger_index, node_configuration.into())
            .await?;

        if let Some(latest) = self
            .db
            .collection::<ProtocolUpdateCollection>()
            .get_latest_protocol_parameters()
            .await?
        {
            let protocol_parameters = chronicle::types::tangle::ProtocolParameters::from(protocol_parameters);
            if latest.parameters.network_name != protocol_parameters.network_name {
                return Err(InxWorkerError::NetworkChanged(
                    latest.parameters.network_name,
                    protocol_parameters.network_name,
                ));
            }
            debug!("Found matching network in the database.");
            if latest.parameters != protocol_parameters {
                debug!("Updating protocol parameters.");
                self.db
                    .collection::<ProtocolUpdateCollection>()
                    .insert_protocol_parameters(start_index, protocol_parameters)
                    .await?;
            }
        } else {
            self.db.clear().await?;
            info!("Reading unspent outputs.");
            let unspent_output_stream = inx
                .read_unspent_outputs()
                .instrument(trace_span!("inx_read_unspent_outputs"))
                .await?;

            let mut count = 0;
            let mut tasks = unspent_output_stream
                .inspect(|_| count += 1)
                .map(|res| Ok(res?.output))
                .try_chunks(INSERT_BATCH_SIZE)
                // We only care if we had an error, so discard the other data
                .map_err(|e| InxWorkerError::Inx(e.1))
                // Convert batches to tasks
                .try_fold(JoinSet::new(), |mut tasks, batch| async {
                    let db = self.db.clone();
                    tasks.spawn(async move { insert_unspent_outputs(&db, &batch).await });
                    Ok(tasks)
                })
                .await?;

            while let Some(res) = tasks.join_next().await {
                // Panic: Acceptable risk
                res.unwrap()?;
            }

            info!("Inserted {} unspent outputs.", count);

            info!(
                "Linking database `{}` to network `{}`.",
                self.db.name(),
                protocol_parameters.network_name()
            );

            self.db
                .collection::<ProtocolUpdateCollection>()
                .insert_protocol_parameters(start_index, protocol_parameters.into())
                .await?;
        }

        metrics::describe_histogram!(
            METRIC_MILESTONE_SYNC_TIME,
            metrics::Unit::Seconds,
            "the time it took to sync the last milestone"
        );
        metrics::describe_gauge!(METRIC_MILESTONE_INDEX, "the last milestone index");
        metrics::describe_gauge!(METRIC_MILESTONE_TIMESTAMP, "the last milestone timestamp");

        Ok((start_index, inx))
    }

    #[instrument(skip_all, fields(milestone_index, created, consumed), err, level = "debug")]
    async fn handle_ledger_update(
        &mut self,
        inx: &mut Inx,
        start_marker: LedgerUpdateMessage,
        stream: &mut (impl futures::Stream<Item = Result<LedgerUpdateMessage, InxError>> + Unpin),
    ) -> Result<(), InxWorkerError> {
        let start_time = std::time::Instant::now();

        let MarkerMessage {
            milestone_index,
            consumed_count,
            created_count,
        } = start_marker.begin().ok_or(InxWorkerError::InvalidMilestoneState)?;

        trace!(
            "Received begin marker of milestone {milestone_index} with {consumed_count} consumed and {created_count} created outputs."
        );

        let mut analytics = Analytics::default();
        let prev_analytics = match self.analytics.take() {
            Some(res) => res,
            None => self.db.get_all_analytics(milestone_index - 1).await?,
        };

        // Only want to accumulate some of the analytics.
        analytics.storage_deposits = prev_analytics.storage_deposits;
        analytics.unspent_outputs = prev_analytics.unspent_outputs;
        let analytics = Arc::new(Mutex::new(analytics.processor()));

        let mut tasks = JoinSet::new();
        let mut actual_created_count = 0;
        let mut actual_consumed_count = 0;

        stream
            .by_ref()
            .take(consumed_count)
            .map(|res| res?.consumed().ok_or(InxWorkerError::InvalidMilestoneState))
            .inspect_ok(|_| {
                actual_consumed_count += 1;
            })
            .try_chunks(INSERT_BATCH_SIZE)
            // We only care if we had an error, so discard the other data
            .map_err(|e| e.1)
            // Convert batches to tasks
            .try_fold(&mut tasks, |tasks, batch| async {
                let db = self.db.clone();
                let analytics = analytics.clone();
                tasks.spawn(async move {
                    analytics.lock().await.process_consumed_outputs(&batch);
                    update_spent_outputs(&db, &batch).await
                });
                Result::<_, InxWorkerError>::Ok(tasks)
            })
            .await?;

        stream
            .by_ref()
            .take(created_count)
            .map(|res| res?.created().ok_or(InxWorkerError::InvalidMilestoneState))
            .inspect_ok(|_| {
                actual_created_count += 1;
            })
            .try_chunks(INSERT_BATCH_SIZE)
            // We only care if we had an error, so discard the other data
            .map_err(|e| e.1)
            // Convert batches to tasks
            .try_fold(&mut tasks, |tasks, batch| async {
                let db = self.db.clone();
                let analytics = analytics.clone();
                tasks.spawn(async move {
                    analytics.lock().await.process_created_outputs(&batch);
                    insert_unspent_outputs(&db, &batch).await
                });
                Result::<_, InxWorkerError>::Ok(tasks)
            })
            .await?;

        while let Some(res) = tasks.join_next().await {
            // Panic: Acceptable risk
            res.unwrap()?;
        }

        let MarkerMessage {
            milestone_index,
            consumed_count,
            created_count,
        } = stream
            .try_next()
            .await?
            .and_then(LedgerUpdateMessage::end)
            .ok_or(InxWorkerError::InvalidMilestoneState)?;
        trace!(
            "Received end of milestone {milestone_index} with {consumed_count} consumed and {created_count} created outputs."
        );
        if actual_created_count != created_count || actual_consumed_count != consumed_count {
            return Err(InxWorkerError::InvalidLedgerUpdateCount {
                received: actual_consumed_count + actual_created_count,
                expected: consumed_count + created_count,
            });
        }

        // Record the result as part of the current span.
        tracing::Span::current().record("milestone_index", milestone_index.0);
        tracing::Span::current().record("created", created_count);
        tracing::Span::current().record("consumed", consumed_count);

        self.handle_cone_stream(inx, &analytics, milestone_index).await?;
        self.handle_protocol_params(inx, milestone_index).await?;
        self.handle_node_configuration(inx, milestone_index).await?;

        // This acts as a checkpoint for the syncing and has to be done last, after everything else completed.
        self.handle_milestone(
            inx,
            milestone_index,
            // Panic: Cannot fail as all other references are held by the tasks and we await them above.
            Arc::try_unwrap(analytics).unwrap().into_inner().finish(),
        )
        .await?;

        let elapsed = start_time.elapsed();

        metrics::histogram!(METRIC_MILESTONE_SYNC_TIME, elapsed);

        Ok(())
    }

    #[instrument(skip_all, level = "trace")]
    async fn handle_protocol_params(
        &self,
        inx: &mut Inx,
        milestone_index: MilestoneIndex,
    ) -> Result<(), InxWorkerError> {
        let parameters = inx
            .read_protocol_parameters(milestone_index.0.into())
            .await?
            .params
            .inner(&())?;

        self.db
            .collection::<ProtocolUpdateCollection>()
            .update_latest_protocol_parameters(milestone_index, parameters.into())
            .await?;

        Ok(())
    }

    #[instrument(skip_all, level = "trace")]
    async fn handle_node_configuration(
        &self,
        inx: &mut Inx,
        milestone_index: MilestoneIndex,
    ) -> Result<(), InxWorkerError> {
        let node_configuration = inx.read_node_configuration().await?;

        self.db
            .collection::<ConfigurationUpdateCollection>()
            .update_latest_node_configuration(milestone_index, node_configuration.into())
            .await?;

        Ok(())
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn handle_milestone(
        &mut self,
        inx: &mut Inx,
        milestone_index: MilestoneIndex,
        analytics: Analytics,
    ) -> Result<(), InxWorkerError> {
        let milestone = inx.read_milestone(milestone_index.0.into()).await?;

        let milestone_index: MilestoneIndex = milestone.milestone_info.milestone_index;

        let milestone_timestamp = milestone.milestone_info.milestone_timestamp.into();

        self.influx_db
            .insert_all_analytics(milestone_timestamp, milestone_index, analytics.clone())
            .await?;

        self.analytics.replace(analytics);

        let milestone_id = milestone
            .milestone_info
            .milestone_id
            .ok_or(InxWorkerError::MissingMilestoneInfo(milestone_index))?;

        let payload =
            if let iota_types::block::payload::Payload::Milestone(payload) = milestone.milestone.inner_unverified()? {
                chronicle::types::stardust::block::payload::MilestonePayload::from(payload)
            } else {
                // The raw data is guaranteed to contain a milestone payload.
                unreachable!();
            };

        self.db
            .collection::<MilestoneCollection>()
            .insert_milestone(milestone_id, milestone_index, milestone_timestamp, payload)
            .await?;

        metrics::gauge!(METRIC_MILESTONE_INDEX, milestone_index.0 as f64);
        metrics::gauge!(METRIC_MILESTONE_TIMESTAMP, milestone_timestamp.0 as f64);

        Ok(())
    }

    #[instrument(skip(self), err, level = "trace")]
    async fn handle_cone_stream(
        &mut self,
        inx: &mut Inx,
        analytics: &Arc<Mutex<AnalyticsProcessor>>,
        milestone_index: MilestoneIndex,
    ) -> Result<(), InxWorkerError> {
        let cone_stream = inx.read_milestone_cone(milestone_index.0.into()).await?;

        let mut tasks = cone_stream
            .map(|res| {
                let BlockWithMetadataMessage { block, metadata } = res?;
                Result::<_, InxWorkerError>::Ok((
                    metadata.block_id,
                    block.clone().inner_unverified()?.into(),
                    block.data(),
                    BlockMetadata::from(metadata),
                ))
            })
            .try_chunks(INSERT_BATCH_SIZE)
            .map_err(|e| e.1)
            .try_fold(JoinSet::new(), |mut tasks, batch| async {
                let db = self.db.clone();
                let analytics = analytics.clone();
                tasks.spawn(async move {
                    let payloads = batch
                        .iter()
                        .filter_map(|(_, block, _, metadata): &(BlockId, Block, Vec<u8>, BlockMetadata)| {
                            if metadata.inclusion_state == LedgerInclusionState::Included {
                                if let Some(Payload::TreasuryTransaction(payload)) = &block.payload {
                                    return Some((
                                        metadata.referenced_by_milestone_index,
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
                    analytics
                        .lock()
                        .await
                        .process_blocks(batch.iter().map(|(_, block, _, metadata)| (block, metadata)));
                    db.collection::<BlockCollection>()
                        .insert_blocks_with_metadata(batch)
                        .await?;
                    Result::<_, InxWorkerError>::Ok(())
                });
                Ok(tasks)
            })
            .await?;

        while let Some(res) = tasks.join_next().await {
            // Panic: Acceptable risk
            res.unwrap()?;
        }

        Ok(())
    }
}

#[instrument(skip_all, err, fields(num = outputs.len()), level = "trace")]
async fn insert_unspent_outputs(db: &MongoDb, outputs: &[LedgerOutput]) -> Result<(), InxWorkerError> {
    let output_collection = db.collection::<OutputCollection>();
    let ledger_collection = db.collection::<LedgerUpdateCollection>();
    try_join! {
        async {
            output_collection.insert_unspent_outputs(outputs).await?;
            Result::<_, InxWorkerError>::Ok(())
        },
        async {
            ledger_collection.insert_unspent_ledger_updates(outputs).await?;
            Ok(())
        }
    }?;
    Ok(())
}

#[instrument(skip_all, err, fields(num = outputs.len()), level = "trace")]
async fn update_spent_outputs(db: &MongoDb, outputs: &[LedgerSpent]) -> Result<(), InxWorkerError> {
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
