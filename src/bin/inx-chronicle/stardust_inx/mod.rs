// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod chunks;
mod config;
mod error;
mod stream;

use async_trait::async_trait;
use bee_inx::client::Inx;
use chronicle::{
    db::{
        collections::{
            BlockCollection, LedgerUpdateCollection, MilestoneCollection, OutputCollection, ProtocolUpdateCollection,
            TreasuryCollection,
        },
        MongoDb,
    },
    runtime::{Actor, ActorContext, HandleEvent},
    types::{
        ledger::{BlockMetadata, LedgerInclusionState, LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::{Block, BlockId, Payload},
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};
use futures::{StreamExt, TryStreamExt};
use tokio::try_join;
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

    pub async fn connect(inx_config: &InxConfig) -> Result<Inx, InxError> {
        let url = url::Url::parse(&inx_config.connect_url)?;

        if url.scheme() != "http" {
            return Err(InxError::InvalidAddress(inx_config.connect_url.clone()));
        }

        for i in 0..inx_config.connection_retry_count {
            match Inx::connect(inx_config.connect_url.clone()).await {
                Ok(inx_client) => return Ok(inx_client),
                Err(_) => {
                    warn!(
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

    #[instrument(skip_all, err, level = "trace")]
    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        info!("Connecting to INX at bind address `{}`.", &self.config.connect_url);
        let mut inx = Self::connect(&self.config).await?;
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
            if node_status.tangle_pruning_index > latest_milestone.0 {
                return Err(InxError::SyncMilestoneGap {
                    start: latest_milestone + 1,
                    end: node_status.tangle_pruning_index.into(),
                });
            } else if node_status.confirmed_milestone.milestone_info.milestone_index < latest_milestone.0 {
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

        let protocol_parameters: ProtocolParameters = inx
            .read_protocol_parameters(start_index.0.into())
            .await?
            .inner()?
            .into();

        debug!("Connected to network `{}`.", protocol_parameters.network_name);

        if let Some(latest) = self
            .db
            .collection::<ProtocolUpdateCollection>()
            .get_latest_protocol_parameters()
            .await?
        {
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

            let (tasks, count) = unspent_output_stream
                // Convert to `LedgerOutput`
                .map(|res| Ok(res?.output.try_into()?))
                // Break into chunks
                .try_chunks(INSERT_BATCH_SIZE)
                // We only care if we had an error, so discard the other data
                .map_err(|e| e.1)
                // Convert batches to tasks
                .map_ok(|batch| {
                    let db = self.db.clone();
                    (
                        batch.len(),
                        tokio::spawn(async move { insert_unspent_outputs(&db, batch).await }),
                    )
                })
                // Fold everything into a total count and list of tasks
                .try_fold((Vec::new(), 0), |(mut tasks, count), (batch_size, task)| async move {
                    tasks.push(task);
                    Result::<_, InxError>::Ok((tasks, count + batch_size))
                })
                .instrument(trace_span!("initial_insert_unspent_outputs"))
                .await?;

            for task in tasks {
                // Panic: Acceptable risk
                task.await.unwrap()?;
            }

            info!("Inserted {} unspent outputs.", count);

            info!(
                "Linking database `{}` to network `{}`.",
                self.db.name(),
                protocol_parameters.network_name
            );

            self.db
                .collection::<ProtocolUpdateCollection>()
                .insert_protocol_parameters(start_index, protocol_parameters)
                .await?;
        }

        cx.add_stream(LedgerUpdateStream::new(
            inx.listen_to_ledger_updates((start_index.0..).into()).await?,
        ));

        debug!("Started listening to ledger updates via INX.");

        metrics::describe_histogram!(
            METRIC_MILESTONE_SYNC_TIME,
            metrics::Unit::Seconds,
            "the time it took to sync the last milestone"
        );
        metrics::describe_gauge!(METRIC_MILESTONE_INDEX, "the last milestone index");
        metrics::describe_gauge!(METRIC_MILESTONE_TIMESTAMP, "the last milestone timestamp");

        Ok(inx)
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Inx Worker".into()
    }
}

#[derive(Debug)]
pub struct LedgerUpdateRecord {
    milestone_index: MilestoneIndex,
    created: Vec<LedgerOutput>,
    consumed: Vec<LedgerSpent>,
}

#[async_trait]
impl HandleEvent<Result<LedgerUpdateRecord, InxError>> for InxWorker {
    #[instrument(
        skip_all,
        fields(milestone_index, created, consumed),
        err,
        level = "debug",
        name = "handle_ledger_update"
    )]
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        ledger_update_result: Result<LedgerUpdateRecord, InxError>,
        inx: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let start_time = std::time::Instant::now();

        let ledger_update = ledger_update_result?;

        // Record the result as part of the current span.
        tracing::Span::current().record("milestone_index", ledger_update.milestone_index.0);
        tracing::Span::current().record("created", &ledger_update.created.len());
        tracing::Span::current().record("consumed", &ledger_update.consumed.len());

        insert_unspent_outputs(&self.db, ledger_update.created).await?;
        update_spent_outputs(&self.db, ledger_update.consumed).await?;

        handle_cone_stream(&self.db, inx, ledger_update.milestone_index).await?;
        handle_protocol_params(&self.db, inx, ledger_update.milestone_index).await?;

        // This acts as a checkpoint for the syncing and has to be done last, after everything else completed.
        handle_milestone(&self.db, inx, ledger_update.milestone_index).await?;

        let elapsed = start_time.elapsed();

        metrics::histogram!(METRIC_MILESTONE_SYNC_TIME, elapsed);

        Ok(())
    }
}

#[instrument(skip_all, fields(num = outputs.len()), level = "trace")]
async fn insert_unspent_outputs(db: &MongoDb, outputs: Vec<LedgerOutput>) -> Result<(), InxError> {
    let output_collection = db.collection::<OutputCollection>();
    let ledger_collection = db.collection::<LedgerUpdateCollection>();
    try_join! {
        async {
            for batch in &outputs.iter().chunks(INSERT_BATCH_SIZE) {
                output_collection.insert_unspent_outputs(batch).await?;
            }
            Result::<_, InxError>::Ok(())
        },
        async {
            for batch in &outputs.iter().chunks(INSERT_BATCH_SIZE) {
                ledger_collection.insert_unspent_ledger_updates(batch).await?;
            }
            Ok(())
        }
    }?;
    Ok(())
}

#[instrument(skip_all, fields(num = outputs.len()), level = "trace")]
async fn update_spent_outputs(db: &MongoDb, outputs: Vec<LedgerSpent>) -> Result<(), InxError> {
    let output_collection = db.collection::<OutputCollection>();
    let ledger_collection = db.collection::<LedgerUpdateCollection>();
    try_join! {
        async {
            for batch in &outputs.iter().chunks(INSERT_BATCH_SIZE) {
                output_collection.update_spent_outputs(batch).await?;
            }
            Result::<_, InxError>::Ok(())
        },
        async {
            for batch in &outputs.iter().chunks(INSERT_BATCH_SIZE) {
                ledger_collection.insert_spent_ledger_updates(batch).await?;
            }
            Ok(())
        }
    }?;
    Ok(())
}

#[instrument(skip_all, level = "trace")]
async fn handle_protocol_params(db: &MongoDb, inx: &mut Inx, milestone_index: MilestoneIndex) -> Result<(), InxError> {
    let parameters: ProtocolParameters = inx
        .read_protocol_parameters(milestone_index.0.into())
        .await?
        .inner()?
        .into();

    db.collection::<ProtocolUpdateCollection>()
        .update_latest_protocol_parameters(milestone_index, parameters)
        .await?;

    Ok(())
}

#[instrument(skip_all, level = "trace")]
async fn handle_milestone(db: &MongoDb, inx: &mut Inx, milestone_index: MilestoneIndex) -> Result<(), InxError> {
    let milestone = inx.read_milestone(milestone_index.0.into()).await?;

    let milestone_index: MilestoneIndex = milestone.milestone_info.milestone_index.into();

    let milestone_timestamp = milestone.milestone_info.milestone_timestamp.into();
    let milestone_id = milestone
        .milestone_info
        .milestone_id
        .ok_or(InxError::MissingMilestoneInfo(milestone_index))?
        .into();
    let payload = Into::into(
        &milestone
            .milestone
            .ok_or(InxError::MissingMilestoneInfo(milestone_index))?,
    );

    db.collection::<MilestoneCollection>()
        .insert_milestone(milestone_id, milestone_index, milestone_timestamp, payload)
        .await?;

    metrics::gauge!(METRIC_MILESTONE_INDEX, milestone_index.0 as f64);
    metrics::gauge!(METRIC_MILESTONE_TIMESTAMP, milestone_timestamp.0 as f64);

    Ok(())
}

#[instrument(skip(db, inx), level = "trace")]
async fn handle_cone_stream(db: &MongoDb, inx: &mut Inx, milestone_index: MilestoneIndex) -> Result<(), InxError> {
    let cone_stream = inx.read_milestone_cone(milestone_index.0.into()).await?;

    let blocks_with_metadata = cone_stream
        .map(|res| {
            let bee_inx::BlockWithMetadata { block, metadata } = res?;
            Result::<_, InxError>::Ok((
                BlockId::from(metadata.block_id),
                Block::from(block.clone().inner()?),
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
    let treasury_payloads = blocks_with_metadata
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

    for batch in &treasury_payloads.into_iter().chunks(INSERT_BATCH_SIZE) {
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
