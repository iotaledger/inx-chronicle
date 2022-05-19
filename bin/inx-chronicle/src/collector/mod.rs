// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
pub mod solidifier;
#[cfg(all(feature = "stardust", feature = "inx"))]
pub(crate) mod stardust_inx;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, HandleEvent, Report, RuntimeError},
};
use mongodb::bson::document::ValueAccessError;
use thiserror::Error;

pub use self::config::CollectorConfig;
use self::solidifier::Solidifier;

#[cfg(feature = "metrics")]
lazy_static::lazy_static! {
    static ref SOLID_COUNTER: bee_metrics::metrics::counter::Counter = Default::default();
}

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    ValueAccess(#[from] ValueAccessError),
}

#[derive(Debug)]
pub struct Collector {
    db: MongoDb,
    config: CollectorConfig,
}

impl Collector {
    pub fn new(db: MongoDb, config: CollectorConfig) -> Self {
        Self { db, config }
    }
}

#[async_trait]
impl Actor for Collector {
    type State = ();
    type Error = CollectorError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        #[cfg(feature = "metrics")]
        cx.addr::<crate::metrics::MetricsWorker>()
            .await
            .send(crate::metrics::RegisterMetric {
                name: "solid_count".to_string(),
                help: "Count of solidified milestones".to_string(),
                metric: SOLID_COUNTER.clone(),
            })?;

        cx.spawn_child(Solidifier::new(0, self.db.clone())).await;
        #[cfg(all(feature = "stardust", feature = "inx"))]
        cx.spawn_child(stardust_inx::InxWorker::new(self.db.clone(), self.config.inx.clone()))
            .await;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<Solidifier>> for Collector {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Solidifier>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    #[cfg(all(feature = "stardust", feature = "inx"))]
                    solidifier::SolidifierError::MissingStardustInxRequester => {
                        cx.spawn_child(report.actor).await;
                    }
                    // TODO: Maybe map Solidifier errors to Collector errors and return them?
                    _ => {
                        cx.shutdown();
                    }
                },
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[cfg(all(feature = "stardust", feature = "inx"))]
mod _stardust_inx {
    use chronicle::{
        db::model::stardust::{message::MessageRecord, milestone::MilestoneRecord},
        types::ledger::Metadata,
    };

    use super::{
        stardust_inx::{InxWorker, InxWorkerError, RequestedMessage},
        *,
    };
    use crate::collector::stardust_inx::MilestoneState;

    #[async_trait]
    impl HandleEvent<Report<InxWorker>> for Collector {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            event: Report<InxWorker>,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match event {
                Report::Success(_) => {
                    cx.shutdown();
                }
                Report::Error(report) => match report.error {
                    ActorError::Result(e) => match e {
                        InxWorkerError::ConnectionError(_) => {
                            let wait_interval = self.config.inx.connection_retry_interval;
                            log::info!("Retrying INX connection in {} seconds.", wait_interval.as_secs_f32());
                            cx.delay(
                                chronicle::runtime::SpawnActor::new(InxWorker::new(
                                    self.db.clone(),
                                    self.config.inx.clone(),
                                )),
                                wait_interval,
                            )?;
                        }
                        // TODO: This is stupid, but we can't use the ErrorKind enum so :shrug:
                        InxWorkerError::TransportFailed(e) => match e.to_string().as_ref() {
                            "transport error" => {
                                cx.spawn_child(InxWorker::new(self.db.clone(), self.config.inx.clone()))
                                    .await;
                            }
                            _ => {
                                cx.shutdown();
                            }
                        },
                        InxWorkerError::MissingCollector => {
                            cx.delay(
                                chronicle::runtime::SpawnActor::new(InxWorker::new(
                                    self.db.clone(),
                                    self.config.inx.clone(),
                                )),
                                None,
                            )?;
                        }
                        InxWorkerError::MongoDb(e) => {
                            return Err(CollectorError::MongoDb(e));
                        }
                        InxWorkerError::ListenerError(_)
                        | InxWorkerError::Runtime(_)
                        | InxWorkerError::Read(_)
                        | InxWorkerError::ParsingAddressFailed(_)
                        | InxWorkerError::InvalidAddress(_) => {
                            cx.shutdown();
                        }
                    },
                    ActorError::Panic | ActorError::Aborted => {
                        cx.shutdown();
                    }
                },
            }
            Ok(())
        }
    }

    #[async_trait]
    impl HandleEvent<inx::proto::Message> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            message: inx::proto::Message,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!(
                "Received Stardust Message Event ({})",
                prefix_hex::encode(message.message_id.as_ref().unwrap().id.as_slice())
            );
            match MessageRecord::try_from(message) {
                Ok(rec) => {
                    self.db.upsert_message_record(&rec).await?;
                }
                Err(e) => {
                    log::error!("Could not read message: {:?}", e);
                }
            };
            Ok(())
        }
    }

    #[async_trait]
    impl HandleEvent<inx::proto::MessageMetadata> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            metadata: inx::proto::MessageMetadata,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!(
                "Received Stardust Message Referenced Event ({})",
                metadata.milestone_index
            );
            match inx::MessageMetadata::try_from(metadata) {
                Ok(rec) => {
                    let message_id = rec.message_id;
                    self.db
                        .update_message_metadata(&message_id.into(), &Metadata::from(rec))
                        .await?;
                }
                Err(e) => {
                    log::error!("Could not read message metadata: {:?}", e);
                }
            };
            Ok(())
        }
    }

    #[async_trait]
    impl HandleEvent<inx::proto::Milestone> for Collector {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            milestone: inx::proto::Milestone,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!(
                "Received Stardust Milestone Event ({})",
                milestone.milestone_info.as_ref().unwrap().milestone_index
            );
            match MilestoneRecord::try_from(milestone) {
                Ok(rec) => {
                    self.db.upsert_milestone_record(&rec).await?;
                    // Get or create the milestone state
                    let mut ms_state = MilestoneState::new(rec.milestone_index);
                    ms_state
                        .process_queue
                        .extend(Vec::from(rec.payload.essence.parents).into_iter());
                    cx.addr::<Solidifier>().await.send(ms_state)?;
                }
                Err(e) => {
                    log::error!("Could not read milestone: {:?}", e);
                }
            }
            Ok(())
        }
    }

    #[async_trait]
    impl<Sender: Actor> HandleEvent<RequestedMessage<Sender>> for Collector
    where
        Sender: 'static + HandleEvent<MilestoneState>,
    {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            RequestedMessage {
                raw,
                metadata,
                sender_addr,
                ms_state,
            }: RequestedMessage<Sender>,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match raw {
                Some(raw) => {
                    log::trace!("Received Stardust Requested Message and Metadata");
                    match MessageRecord::try_from((raw, metadata)) {
                        Ok(rec) => {
                            self.db.upsert_message_record(&rec).await?;
                            // Send this directly to the solidifier that requested it
                            sender_addr.send(ms_state)?;
                        }
                        Err(e) => {
                            log::error!("Could not read message: {:?}", e);
                        }
                    };
                }
                None => {
                    log::trace!("Received Stardust Requested Metadata");
                    match inx::MessageMetadata::try_from(metadata) {
                        Ok(rec) => {
                            let message_id = rec.message_id;
                            self.db
                                .update_message_metadata(&message_id.into(), &Metadata::from(rec))
                                .await?;
                            // Send this directly to the solidifier that requested it
                            sender_addr.send(ms_state)?;
                        }
                        Err(e) => {
                            log::error!("Could not read message metadata: {:?}", e);
                        }
                    };
                }
            }

            Ok(())
        }
    }
}
