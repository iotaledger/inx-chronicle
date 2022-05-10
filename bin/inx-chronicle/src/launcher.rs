// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{
            context::ActorContext, error::ActorError, event::HandleEvent, report::Report, util::SpawnActor, Actor,
        },
        error::RuntimeError,
    },
};
use clap::Parser;
use inx::NodeStatus;
use mongodb::error::ErrorKind;
use thiserror::Error;

#[cfg(feature = "api")]
use crate::api::ApiWorker;
#[cfg(feature = "inx")]
use crate::inx::{InxRequest, InxWorker, InxWorkerError};
use crate::{
    cli::CliArgs,
    collector::{Collector, CollectorError},
    config::{ChronicleConfig, ConfigError},
    syncer::{self, Syncer},
};

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Debug, Default)]
/// Supervisor actor
pub struct Launcher {
    node_status: Option<NodeStatus>,
}

#[async_trait]
impl Actor for Launcher {
    type State = (ChronicleConfig, MongoDb);
    type Error = LauncherError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        // Get configuration.
        let cli_args = CliArgs::parse();
        let mut config = match &cli_args.config {
            Some(path) => ChronicleConfig::from_file(path)?,
            None => {
                if let Ok(path) = std::env::var("CONFIG_PATH") {
                    ChronicleConfig::from_file(path)?
                } else {
                    ChronicleConfig::default()
                }
            }
        };
        config.apply_cli_args(cli_args);

        // Connect to MongoDb instance.
        let db = MongoDb::connect(&config.mongodb).await?;

        if let Some(node_status) = db.status().await? {
            log::info!("{:?}", node_status);
        } else {
            log::info!("No node status has been found in the database, it seems like the database is empty.");
        };

        // Start Collector.
        cx.spawn_child(Collector::new(db.clone(), 10)).await;

        // Start InxWorker.
        #[cfg(feature = "inx")]
        {
            cx.spawn_child(InxWorker::new(config.inx.clone())).await;

            // Send a `NodeStatus` request to the `InxWorker`
            #[cfg(feature = "inx")]
            cx.addr::<InxWorker>().await.send(InxRequest::NodeStatus)?;
        }

        // Start ApiWorker.
        #[cfg(feature = "api")]
        cx.spawn_child(ApiWorker::new(db.clone(), config.api.clone())).await;

        // Start Syncer.
        cx.spawn_child(Syncer::new(db.clone(), config.syncer.clone())).await;

        Ok((config, db))
    }
}

#[async_trait]
impl HandleEvent<Report<Collector>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Collector>,
        (config, db): &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    CollectorError::MongoDb(e) => match e.kind.as_ref() {
                        // Only a few possible errors we could potentially recover from
                        ErrorKind::Io(_) | ErrorKind::ServerSelection { message: _, .. } => {
                            let new_db = MongoDb::connect(&config.mongodb).await?;
                            *db = new_db;
                            cx.spawn_child(Collector::new(db.clone(), 1)).await;
                        }
                        _ => {
                            cx.shutdown();
                        }
                    },
                    _ => {
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

#[cfg(feature = "inx")]
#[async_trait]
impl HandleEvent<Report<InxWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxWorker>,
        (config, _): &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match &e.error {
                ActorError::Result(e) => match e {
                    InxWorkerError::ConnectionError(_) => {
                        let wait_interval = config.inx.connection_retry_interval;
                        log::info!("Retrying INX connection in {} seconds.", wait_interval.as_secs_f32());
                        cx.delay(SpawnActor::new(InxWorker::new(config.inx.clone())), wait_interval)?;
                    }
                    InxWorkerError::InvalidAddress(_) => {
                        cx.shutdown();
                    }
                    InxWorkerError::ParsingAddressFailed(_) => {
                        cx.shutdown();
                    }
                    // TODO: This is stupid, but we can't use the ErrorKind enum so :shrug:
                    InxWorkerError::TransportFailed(e) => match e.to_string().as_ref() {
                        "transport error" => {
                            cx.spawn_child(InxWorker::new(config.inx.clone())).await;
                        }
                        _ => {
                            cx.shutdown();
                        }
                    },
                    InxWorkerError::Read(_) => {
                        cx.shutdown();
                    }
                    InxWorkerError::Runtime(_) => {
                        cx.shutdown();
                    }
                    InxWorkerError::ListenerError(_) => {
                        cx.shutdown();
                    }
                    InxWorkerError::MissingCollector => {
                        cx.delay(SpawnActor::new(InxWorker::new(config.inx.clone())), None)?;
                    }
                    InxWorkerError::FailedToAnswerRequest => {
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

#[cfg(feature = "api")]
#[async_trait]
impl HandleEvent<Report<ApiWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<ApiWorker>,
        (config, db): &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let new_db = MongoDb::connect(&config.mongodb).await?;
                    *db = new_db;
                    cx.spawn_child(ApiWorker::new(db.clone(), config.api.clone())).await;
                }
                ActorError::Panic | ActorError::Aborted => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<Syncer>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        report: Report<Syncer>,
        (_config, _db): &mut Self::State,
    ) -> Result<(), Self::Error> {
        match report {
            Report::Success(_) => log::info!("Syncer finished."),
            Report::Error(_) => log::error!("Syncer finished with errors."),
        }
        cx.shutdown();
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<NodeStatus> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        node_status: NodeStatus,
        _: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // Start syncing from the node's pruning index to it's current ledger index + X.
        // NOTE: we make sure there's a bit of overlap from which Chronicle started to listen
        // to the stream of live milestones.
        if self.node_status.is_none() {
            let start_index = node_status.pruning_index + 1;
            let end_index = node_status.ledger_index + 10;

            cx.addr::<Syncer>().await.send(syncer::Stop(end_index))?;
            cx.addr::<Syncer>().await.send(syncer::Next(start_index))?;
        }

        self.node_status.replace(node_status);

        Ok(())
    }
}
