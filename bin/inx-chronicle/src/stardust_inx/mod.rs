// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;

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
use mongodb::ClientSession;
use pin_project::pin_project;
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

pub struct InxWorkerState {
    inx: Inx,
    unspent_cutoff: MilestoneIndex,
}

#[async_trait]
impl Actor for InxWorker {
    type State = InxWorkerState;
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
        }) = self.db.get_newest_milestone().await?
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

        debug!("Connected to network `{}`.", protocol_parameters.network_name);

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
            info!(
                "Linking database `{}` to network `{}`.",
                self.db.name(),
                protocol_parameters.network_name
            );

            self.db
                .insert_protocol_parameters(&mut session, start_index, protocol_parameters)
                .await?;

            info!("Reading unspent outputs.");
            let mut unspent_output_stream = inx.read_unspent_outputs().await?;

            let mut outputs = Vec::new();
            while let Some(bee_inx::UnspentOutput { output, .. }) = unspent_output_stream.try_next().await? {
                trace!("Received unspent output: {}", output.block_id);
                outputs.push(output.try_into()?);
            }
            info!("Inserting {} unspent outputs.", outputs.len());

            self.db.insert_outputs(&mut session, outputs.iter()).await?;
            self.db.insert_ledger_updates(&mut session, outputs.iter()).await?;
        }

        session.commit_transaction().await?;

        cx.add_stream(LedgerUpdateStream::new(
            inx.listen_to_ledger_updates((start_index.0..).into()).await?,
        ));

        metrics::describe_histogram!(
            METRIC_MILESTONE_SYNC_TIME,
            metrics::Unit::Seconds,
            "the time it took to sync the last milestone"
        );
        metrics::describe_gauge!(METRIC_MILESTONE_INDEX, "the last milestone index");
        metrics::describe_gauge!(METRIC_MILESTONE_TIMESTAMP, "the last milestone timestamp");

        Ok(InxWorkerState {
            inx,
            unspent_cutoff: self
                .db
                .get_latest_unspent_output_metadata()
                .await?
                .map(|o| o.booked.milestone_index)
                .unwrap_or_default(),
        })
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Inx Worker".into()
    }
}

#[derive(Debug)]
pub struct LedgerUpdateRecord {
    milestone_index: MilestoneIndex,
    created: Vec<OutputWithMetadata>,
    consumed: Vec<OutputWithMetadata>,
}

#[pin_project]
pub struct LedgerUpdateStream<S> {
    #[pin]
    inner: S,
    #[pin]
    record: Option<LedgerUpdateRecord>,
}

impl<S> LedgerUpdateStream<S> {
    fn new(inner: S) -> Self {
        Self { inner, record: None }
    }
}

impl<S: Stream<Item = Result<bee_inx::LedgerUpdate, bee_inx::Error>>> Stream for LedgerUpdateStream<S> {
    type Item = Result<LedgerUpdateRecord, InxError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        use bee_inx::LedgerUpdate;

        let mut this = self.project();
        Poll::Ready(loop {
            if let Poll::Ready(next) = this.inner.as_mut().poll_next(cx) {
                if let Some(res) = next {
                    match res {
                        Ok(ledger_update) => match ledger_update {
                            LedgerUpdate::Begin(marker) => {
                                // We shouldn't already have a record. If we do, that's bad.
                                if let Some(record) = this.record.as_mut().take() {
                                    break Some(Err(InxError::InvalidLedgerUpdateCount {
                                        received: record.consumed.len() + record.created.len(),
                                        expected: record.consumed.capacity() + record.created.capacity(),
                                    }));
                                } else {
                                    this.record.set(Some(LedgerUpdateRecord {
                                        milestone_index: marker.milestone_index.into(),
                                        created: Vec::with_capacity(marker.created_count),
                                        consumed: Vec::with_capacity(marker.consumed_count),
                                    }));
                                }
                            }
                            LedgerUpdate::Consumed(consumed) => {
                                if let Some(mut record) = this.record.as_mut().as_pin_mut() {
                                    match OutputWithMetadata::try_from(consumed) {
                                        Ok(consumed) => {
                                            record.consumed.push(consumed);
                                        }
                                        Err(e) => {
                                            break Some(Err(e.into()));
                                        }
                                    }
                                } else {
                                    break Some(Err(InxError::InvalidMilestoneState));
                                }
                            }
                            LedgerUpdate::Created(created) => {
                                if let Some(mut record) = this.record.as_mut().as_pin_mut() {
                                    match OutputWithMetadata::try_from(created) {
                                        Ok(created) => {
                                            record.created.push(created);
                                        }
                                        Err(e) => {
                                            break Some(Err(e.into()));
                                        }
                                    }
                                } else {
                                    break Some(Err(InxError::InvalidMilestoneState));
                                }
                            }
                            LedgerUpdate::End(marker) => {
                                if let Some(record) = this.record.as_mut().take() {
                                    if record.created.len() != marker.created_count
                                        || record.consumed.len() != marker.consumed_count
                                    {
                                        break Some(Err(InxError::InvalidLedgerUpdateCount {
                                            received: record.consumed.len() + record.created.len(),
                                            expected: marker.consumed_count + marker.created_count,
                                        }));
                                    }
                                    break Some(Ok(record));
                                } else {
                                    break Some(Err(InxError::InvalidMilestoneState));
                                }
                            }
                        },
                        Err(e) => {
                            break Some(Err(e.into()));
                        }
                    }
                } else {
                    // If we were supposed to be in the middle of a milestone, something went wrong.
                    if let Some(record) = this.record.as_mut().take() {
                        break Some(Err(InxError::InvalidLedgerUpdateCount {
                            received: record.consumed.len() + record.created.len(),
                            expected: record.consumed.capacity() + record.created.capacity(),
                        }));
                    } else {
                        break None;
                    }
                }
            }
        })
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
        InxWorkerState { inx, unspent_cutoff }: &mut Self::State,
    ) -> Result<(), Self::Error> {
        trace!("Received ledger update event {:#?}", ledger_update_result);

        let start_time = std::time::Instant::now();

        let ledger_update = ledger_update_result?;

        let mut session = self.db.start_transaction(None).await?;

        let outputs = ledger_update.created.iter().chain(ledger_update.consumed.iter());

        if ledger_update.milestone_index > *unspent_cutoff {
            trace!("Batch inserting {}", ledger_update.milestone_index);
            self.db
                .insert_outputs(&mut session, ledger_update.created.iter())
                .await?;
            self.db
                .upsert_outputs(&mut session, ledger_update.consumed.iter())
                .await?;
            self.db.insert_ledger_updates(&mut session, outputs).await?;
        } else {
            trace!("Upserting {}", ledger_update.milestone_index);
            self.db.upsert_outputs(&mut session, outputs.clone()).await?;
            self.db.upsert_ledger_updates(&mut session, outputs).await?;
        }

        self.handle_cone_stream(inx, &mut session, ledger_update.milestone_index)
            .await?;

        session.commit_transaction().await?;

        let elapsed = start_time.elapsed();

        metrics::histogram!(METRIC_MILESTONE_SYNC_TIME, elapsed);

        Ok(())
    }
}

impl InxWorker {
    async fn handle_cone_stream(
        &self,
        inx: &mut Inx,
        session: &mut ClientSession,
        milestone_index: MilestoneIndex,
    ) -> Result<(), InxError> {
        let milestone = inx.read_milestone(milestone_index.0.into()).await?;
        let parameters: ProtocolParameters = inx
            .read_protocol_parameters(milestone_index.0.into())
            .await?
            .inner()?
            .into();

        self.db
            .update_latest_protocol_parameters(session, milestone_index, parameters)
            .await?;

        trace!("Received milestone: `{:?}`", milestone);

        let milestone_index: MilestoneIndex = milestone.milestone_info.milestone_index.into();
        tracing::Span::current().record("milestone_index", milestone_index.0);
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

        let cone_stream = inx.read_milestone_cone(milestone_index.0.into()).await?;

        let blocks_with_metadata = cone_stream
            .map(|res| {
                let bee_inx::BlockWithMetadata { block, metadata } = res?;
                Result::<_, InxError>::Ok((block.clone().inner()?.into(), block.data(), metadata.into()))
            })
            .try_collect::<Vec<_>>()
            .await?;

        self.db
            .insert_blocks_with_metadata(session, blocks_with_metadata)
            .await?;

        self.db
            .insert_milestone(session, milestone_id, milestone_index, milestone_timestamp, payload)
            .await?;

        metrics::gauge!(METRIC_MILESTONE_INDEX, milestone_index.0 as f64);
        metrics::gauge!(METRIC_MILESTONE_TIMESTAMP, milestone_timestamp.0 as f64);

        Ok(())
    }
}
