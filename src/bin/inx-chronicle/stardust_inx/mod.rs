// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod chunks;
mod config;
mod error;
mod stream;

use bee_inx::client::Inx;
use chronicle::{
    db::{
        collections::{
            BlockCollection, MilestoneCollection, OutputCollection, ProtocolUpdateCollection, TreasuryCollection,
        },
        MongoDb,
    },
    types::{
        ledger::{BlockMetadata, LedgerInclusionState, LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::{Block, BlockId, Payload},
        tangle::MilestoneIndex,
    },
};
use futures::{StreamExt, TryStreamExt};
use tokio::task::JoinSet;
use tracing::{debug, info, instrument, trace_span, warn, Instrument};

use self::{chunks::ChunksExt, stream::LedgerUpdateStream};
pub use self::{config::InxConfig, error::InxError};

/// Batch size for insert operations.
pub const INSERT_BATCH_SIZE: usize = 10000;

pub struct InxWorker {
    db: MongoDb,
    config: InxConfig,
}

const METRIC_MILESTONE_INDEX: &str = "milestone_index";
const METRIC_MILESTONE_TIMESTAMP: &str = "milestone_timestamp";
const METRIC_MILESTONE_SYNC_TIME: &str = "milestone_sync_time";

impl InxWorker {
    /// Creates an [`Inx`] client by connecting to the endpoint specified in `inx_config`.
    pub fn new(db: &MongoDb, inx_config: &InxConfig) -> Self {
        Self {
            db: db.clone(),
            config: inx_config.clone(),
        }
    }

    async fn connect(&self) -> Result<Inx, InxError> {
        let url = url::Url::parse(&self.config.connect_url)?;

        if url.scheme() != "http" {
            return Err(InxError::InvalidAddress(self.config.connect_url.clone()));
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
        Err(InxError::ConnectionError)
    }

    pub async fn run(&mut self) -> Result<(), InxError> {
        let (start_index, mut inx) = self.init().await?;

        let mut stream = LedgerUpdateStream::new(inx.listen_to_ledger_updates((start_index.0..).into()).await?);

        debug!("Started listening to ledger updates via INX.");

        while let Some(ledger_update) = stream.try_next().await? {
            handle_ledger_update(ledger_update, &mut inx, &self.db).await?;
        }

        tracing::debug!("INX stream closed unexpectedly.");

        Ok(())
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn init(&mut self) -> Result<(MilestoneIndex, Inx), InxError> {
        info!("Connecting to INX at bind address `{}`.", &self.config.connect_url);
        let mut inx = self.connect().await?;
        info!("Connected to INX.");

        // Request the node status so we can get the pruning index and latest confirmed milestone
        let node_status = inx.read_node_status().await?;

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
                return Err(InxError::SyncMilestoneGap {
                    start: latest_milestone + 1,
                    end: node_status.tangle_pruning_index.into(),
                });
            } else if node_status.confirmed_milestone.milestone_info.milestone_index.0 < latest_milestone.0 {
                return Err(InxError::SyncMilestoneIndexMismatch {
                    node: node_status.confirmed_milestone.milestone_info.milestone_index.into(),
                    db: latest_milestone,
                });
            } else {
                latest_milestone + 1
            }
        } else {
            self.config
                .sync_start_milestone
                .max((node_status.tangle_pruning_index + 1).into())
        };

        let protocol_parameters = inx
            .read_protocol_parameters(start_index.0.into())
            .await?
            .params
            .inner_unverified()?;

        debug!("Connected to network `{}`.", protocol_parameters.network_name());

        if let Some(latest) = self
            .db
            .collection::<ProtocolUpdateCollection>()
            .get_latest_protocol_parameters()
            .await?
        {
            let protocol_parameters = chronicle::types::tangle::ProtocolParameters::from(protocol_parameters);
            if latest.parameters.network_name != protocol_parameters.network_name {
                return Err(InxError::NetworkChanged(
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

            let mut tasks = unspent_output_stream
                .map(|res| LedgerOutput::try_from(res?.output))
                .try_chunks(INSERT_BATCH_SIZE)
                // We only care if we had an error, so discard the other data
                .map_err(|e| InxError::BeeInx(e.1))
                // Convert batches to tasks
                .try_fold(JoinSet::new(), |mut tasks, batch| async {
                    let db = self.db.clone();
                    tasks.spawn(async move { insert_unspent_outputs(&db, batch).await });
                    Ok(tasks)
                })
                .await?;

            let mut count = 0;
            while let Some(res) = tasks.join_next().await {
                // Panic: Acceptable risk
                count += res.unwrap()?;
            }

            self.db.collection::<OutputCollection>().create_ledger_updates().await?;

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
}

#[derive(Debug)]
pub struct LedgerUpdateRecord {
    milestone_index: MilestoneIndex,
    created: Vec<bee_inx::LedgerOutput>,
    consumed: Vec<bee_inx::LedgerSpent>,
}

#[instrument(skip_all, fields(milestone_index, created, consumed), err, level = "debug")]
async fn handle_ledger_update(ledger_update: LedgerUpdateRecord, inx: &mut Inx, db: &MongoDb) -> Result<(), InxError> {
    let start_time = std::time::Instant::now();

    // Record the result as part of the current span.
    tracing::Span::current().record("milestone_index", ledger_update.milestone_index.0);
    tracing::Span::current().record("created", ledger_update.created.len());
    tracing::Span::current().record("consumed", ledger_update.consumed.len());

    insert_unspent_outputs(
        db,
        ledger_update
            .created
            .into_iter()
            .map(|o| o.try_into())
            .collect::<Result<_, _>>()?,
    )
    .await?;
    update_spent_outputs(
        db,
        ledger_update
            .consumed
            .into_iter()
            .map(|o| o.try_into())
            .collect::<Result<_, _>>()?,
    )
    .await?;

    handle_cone_stream(db, inx, ledger_update.milestone_index).await?;
    handle_protocol_params(db, inx, ledger_update.milestone_index).await?;

    // This acts as a checkpoint for the syncing and has to be done last, after everything else completed.
    handle_milestone(db, inx, ledger_update.milestone_index).await?;

    let elapsed = start_time.elapsed();

    metrics::histogram!(METRIC_MILESTONE_SYNC_TIME, elapsed);

    Ok(())
}

#[instrument(skip_all, err, fields(num = outputs.len()), level = "trace")]
async fn insert_unspent_outputs(db: &MongoDb, outputs: Vec<LedgerOutput>) -> Result<usize, InxError> {
    let output_collection = db.collection::<OutputCollection>();
    for batch in &outputs.iter().chunks(INSERT_BATCH_SIZE) {
        output_collection.insert_unspent_outputs(batch).await?;
    }
    Ok(outputs.len())
}

#[instrument(skip_all, err, fields(num = outputs.len()), level = "trace")]
async fn update_spent_outputs(db: &MongoDb, outputs: Vec<LedgerSpent>) -> Result<usize, InxError> {
    let output_collection = db.collection::<OutputCollection>();
    for batch in &outputs.iter().chunks(INSERT_BATCH_SIZE) {
        output_collection.update_spent_outputs(batch).await?;
    }
    Ok(outputs.len())
}

#[instrument(skip_all, level = "trace")]
async fn handle_protocol_params(db: &MongoDb, inx: &mut Inx, milestone_index: MilestoneIndex) -> Result<(), InxError> {
    let parameters = inx
        .read_protocol_parameters(milestone_index.0.into())
        .await?
        .params
        .inner(&())?;

    db.collection::<ProtocolUpdateCollection>()
        .update_latest_protocol_parameters(milestone_index, parameters.into())
        .await?;

    Ok(())
}

#[instrument(skip_all, err, level = "trace")]
async fn handle_milestone(db: &MongoDb, inx: &mut Inx, milestone_index: MilestoneIndex) -> Result<(), InxError> {
    let milestone = inx.read_milestone(milestone_index.0.into()).await?;

    let milestone_index: MilestoneIndex = milestone.milestone_info.milestone_index.into();

    let milestone_timestamp = milestone.milestone_info.milestone_timestamp.into();
    let milestone_id = milestone
        .milestone_info
        .milestone_id
        .ok_or(InxError::MissingMilestoneInfo(milestone_index))?
        .into();

    let payload =
        if let bee_block_stardust::payload::Payload::Milestone(payload) = milestone.milestone.inner_unverified()? {
            chronicle::types::stardust::block::payload::MilestonePayload::from(payload)
        } else {
            // The raw data is guaranteed to contain a milestone payload.
            unreachable!();
        };

    db.collection::<MilestoneCollection>()
        .insert_milestone(milestone_id, milestone_index, milestone_timestamp, payload)
        .await?;

    metrics::gauge!(METRIC_MILESTONE_INDEX, milestone_index.0 as f64);
    metrics::gauge!(METRIC_MILESTONE_TIMESTAMP, milestone_timestamp.0 as f64);

    Ok(())
}

#[instrument(skip(db, inx), err, level = "trace")]
async fn handle_cone_stream(db: &MongoDb, inx: &mut Inx, milestone_index: MilestoneIndex) -> Result<(), InxError> {
    let cone_stream = inx.read_milestone_cone(milestone_index.0.into()).await?;

    let blocks_with_metadata = cone_stream
        .map(|res| {
            let bee_inx::BlockWithMetadata { block, metadata } = res?;
            Result::<_, InxError>::Ok((
                metadata.block_id.into(),
                block.clone().inner_unverified()?.into(),
                block.data(),
                BlockMetadata::from(metadata),
            ))
        })
        .try_collect::<Vec<_>>()
        .await?;

    // Unfortunately, clippy is wrong here. As much as I would love to use the iterator directly
    // rather than collecting, rust is unable to resolve the bounds and cannot adequately express
    // what is actually wrong.
    #[allow(clippy::needless_collect)]
    let payloads = blocks_with_metadata
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

    for batch in &payloads.into_iter().chunks(INSERT_BATCH_SIZE) {
        db.collection::<TreasuryCollection>()
            .insert_treasury_payloads(batch)
            .await?;
    }

    for batch in &blocks_with_metadata.into_iter().chunks(INSERT_BATCH_SIZE) {
        db.collection::<BlockCollection>()
            .insert_blocks_with_metadata(batch)
            .await?;
    }

    Ok(())
}
