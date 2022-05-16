// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{
            context::ActorContext,
            error::ActorError,
            event::HandleEvent,
            report::{ErrorReport, Report},
            util::SpawnActor,
            Actor,
        },
        error::RuntimeError,
    },
};
use clap::Parser;
use mongodb::error::ErrorKind;
use thiserror::Error;

#[cfg(feature = "api")]
use crate::api::ApiWorker;
#[cfg(feature = "inx")]
use crate::inx::{InxRequest, InxWorker, InxWorkerError};
use crate::{
    cli::CliArgs,
    config::{ChronicleConfig, ConfigError},
    inx::collector::{Collector, CollectorError},
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
pub struct Launcher;

pub struct LauncherState {
    config: ChronicleConfig,
    db: MongoDb,
}

#[async_trait]
impl Actor for Launcher {
    type State = LauncherState;
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

        // Start InxWorker.
        #[cfg(feature = "inx")]
        cx.spawn_child(InxWorker::new(db.clone(), config.inx.clone())).await;

        // Start ApiWorker.
        #[cfg(feature = "api")]
        cx.spawn_child(ApiWorker::new(db.clone(), config.api.clone())).await;

        Ok(LauncherState { config, db })
    }
}

#[cfg(feature = "inx")]
#[async_trait]
impl HandleEvent<Report<InxWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxWorker>,
        LauncherState { config, db }: &mut Self::State,
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
                        cx.delay(
                            SpawnActor::new(InxWorker::new(db.clone(), config.inx.clone())),
                            wait_interval,
                        )?;
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
                            cx.spawn_child(InxWorker::new(db.clone(), config.inx.clone())).await;
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
                        cx.delay(SpawnActor::new(InxWorker::new(db.clone(), config.inx.clone())), None)?;
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
        LauncherState { config, db }: &mut Self::State,
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
