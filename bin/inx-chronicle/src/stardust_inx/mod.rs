// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;
mod stream;

use async_trait::async_trait;
use bee_inx::client::Inx;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent},
    types::{
        ledger::{MilestoneIndexTimestamp, OutputWithMetadata},
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};
pub use config::InxConfig;
pub use error::InxError;
use futures::{Stream, StreamExt, TryStreamExt};
use pin_project::pin_project;
use stream::{LedgerUpdateRecord, LedgerUpdateStream};
use tracing::{debug, info, instrument, trace, warn};

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

fn log_corrupt(num: usize, desc: &str) {
    if num > 0 {
        debug!("Removed {num} ledger_updates.");
    };
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

        let db_latest_milestone = self.db.get_newest_milestone().await?;

        // Check if there is an unfixable gap in our node data.
        let start_index = if let Some(MilestoneIndexTimestamp {
            milestone_index: latest_milestone,
            ..
        }) = db_latest_milestone
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

        let inx_protocol_parameters: ProtocolParameters = inx
            .read_protocol_parameters(start_index.0.into())
            .await?
            .inner()?
            .into();

        debug!("Connected to network `{}`.", inx_protocol_parameters.network_name);

        if let Some(latest) = self.db.get_latest_protocol_parameters().await? {
            if latest.parameters.network_name != inx_protocol_parameters.network_name {
                return Err(InxError::NetworkChanged(
                    latest.parameters.network_name,
                    inx_protocol_parameters.network_name,
                ));
            }
            if latest.parameters != inx_protocol_parameters {
                self.db
                    .insert_protocol_parameters(start_index, inx_protocol_parameters)
                    .await?;
            }
        } else {
            info!(
                "Linking database `{}` to network `{}`.",
                self.db.name(),
                inx_protocol_parameters.network_name
            );

            debug!("Checking for corrupt data.");
            log_corrupt(
                self.db.remove_ledger_updates_newer_than_milestone(0.into()).await?,
                "ledger_updates",
            );
            log_corrupt(self.db.remove_outputs_newer_than_milestone(0.into()).await?, "outputs");

            info!("Reading unspent outputs.");
            let mut unspent_output_stream = inx.read_unspent_outputs().await?;

            let mut updates = Vec::new();
            while let Some(bee_inx::UnspentOutput { output, .. }) = unspent_output_stream.try_next().await? {
                updates.push(output.try_into()?);
            }
            info!("Inserting {} unspent outputs.", updates.len());

            // TODO: This should be done concurrently.
            self.db.insert_ledger_updates(updates.iter()).await?;
            self.db.insert_outputs(updates).await?;

            // The protocol parameters act as a checkpoint. Writing the unspent outputs was successful only if the
            // parameters are in the database.
            self.db
                .insert_protocol_parameters(start_index, inx_protocol_parameters)
                .await?;
        }

        // We check and remove potentially corrupt data from previous runs.
        if let Some(MilestoneIndexTimestamp {
            milestone_index: latest,
            ..
        }) = db_latest_milestone
        {
            log_corrupt(
                self.db.remove_ledger_updates_newer_than_milestone(latest).await?,
                "ledger_updates",
            );
            log_corrupt(self.db.remove_outputs_newer_than_milestone(latest).await?, "outputs");
            log_corrupt(self.db.remove_blocks_newer_than_milestone(latest).await?, "blocks");
            log_corrupt(
                self.db.remove_protocol_updates_newer_than_milestone(latest).await?,
                "protocol_updates",
            );
            log_corrupt(self.db.remove_treasury_newer_than_milestone(latest).await?, "trasury");
        }

        let ledger_update_stream =
            LedgerUpdateStream::new(inx.listen_to_ledger_updates((start_index.0..).into()).await?);

        metrics::describe_histogram!(
            METRIC_MILESTONE_SYNC_TIME,
            metrics::Unit::Seconds,
            "the time it took to sync the last milestone"
        );
        metrics::describe_gauge!(METRIC_MILESTONE_INDEX, "the last milestone index");
        metrics::describe_gauge!(METRIC_MILESTONE_TIMESTAMP, "the last milestone timestamp");

        cx.add_stream(ledger_update_stream);

        Ok(inx)
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Inx Worker".into()
    }
}

#[async_trait]
impl HandleEvent<Result<LedgerUpdateRecord, InxError>> for InxWorker {
    #[instrument(
        skip_all,
        fields(milestone_index),
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

        self.db.upsert_ledger_updates(ledger_update.outputs.into_iter()).await?;

        let milestone = inx.read_milestone(ledger_update.milestone_index.0.into()).await?;
        let parameters: ProtocolParameters = inx
            .read_protocol_parameters(ledger_update.milestone_index.0.into())
            .await?
            .inner()?
            .into();

        self.db
            .update_latest_protocol_parameters(ledger_update.milestone_index, parameters)
            .await?;

        let milestone_index: MilestoneIndex = milestone.milestone_info.milestone_index.into();
        tracing::Span::current().record("milestone_index", milestone_index.0);
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

        self.db.insert_blocks_with_metadata(blocks_with_metadata).await?;

        self.db
            .insert_milestone(milestone_id, milestone_index, milestone_timestamp, payload)
            .await?;

        let elapsed = start_time.elapsed();

        metrics::histogram!(METRIC_MILESTONE_SYNC_TIME, elapsed);
        metrics::gauge!(METRIC_MILESTONE_INDEX, milestone_index.0 as f64);
        metrics::gauge!(METRIC_MILESTONE_TIMESTAMP, milestone_timestamp.0 as f64);

        Ok(())
    }
}
