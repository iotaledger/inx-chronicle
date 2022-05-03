// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
pub mod api;
mod broker;
mod cli;
mod config;
#[cfg(feature = "inx")]
mod inx;

use std::error::Error;

use async_trait::async_trait;
#[cfg(feature = "stardust")]
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{
            context::ActorContext, error::ActorError, event::HandleEvent, report::Report, util::SpawnActor, Actor,
        },
        error::RuntimeError,
        scope::RuntimeScope,
        Runtime,
    },
};
use clap::Parser;
use mongodb::error::ErrorKind;
use thiserror::Error;

#[cfg(feature = "api")]
use self::api::ApiWorker;
#[cfg(feature = "inx")]
use self::inx::{InxWorker, InxWorkerError};
use self::{
    broker::{Broker, BrokerError},
    cli::CliArgs,
    config::{ChronicleConfig, ConfigError},
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

#[derive(Debug)]
/// Supervisor actor
pub struct Launcher;

#[async_trait]
impl Actor for Launcher {
    type State = ChronicleConfig;
    type Error = LauncherError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let cli_args = CliArgs::parse();
        let mut config = match &cli_args.config {
            Some(path) => config::ChronicleConfig::from_file(path)?,
            None => {
                if let Ok(path) = std::env::var("CONFIG_PATH") {
                    ChronicleConfig::from_file(path)?
                } else {
                    ChronicleConfig::default()
                }
            }
        };
        config.apply_cli_args(cli_args);

        let db = MongoDb::connect(&config.mongodb).await?;

        if let Some(node_status) = db.status().await? {
            log::info!("{:?}", node_status);
        } else {
            log::info!("No node status has been found in the database, it seems like the database is empty.");
        };

        cx.spawn_child(Broker::new(db.clone())).await;
        #[cfg(feature = "inx")]
        cx.spawn_child(InxWorker::new(config.inx.clone())).await;

        #[cfg(feature = "api")]
        cx.spawn_child(ApiWorker::new(db)).await;
        Ok(config)
    }
}

#[async_trait]
impl HandleEvent<Report<Broker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Broker>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match report.error {
                ActorError::Result(e) => match e {
                    BrokerError::RuntimeError(_) => {
                        cx.shutdown();
                    }
                    BrokerError::MongoDbError(e) => match e.kind.as_ref() {
                        // Only a few possible errors we could potentially recover from
                        ErrorKind::Io(_) | ErrorKind::ServerSelection { message: _, .. } => {
                            let db = MongoDb::connect(&config.mongodb).await?;
                            cx.spawn_child(Broker::new(db)).await;
                        }
                        _ => {
                            cx.shutdown();
                        }
                    },
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
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
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
                    InxWorkerError::MissingBroker => {
                        cx.spawn_child(InxWorker::new(config.inx.clone())).await;
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
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match report.error {
                ActorError::Result(_) => {
                    let db = MongoDb::connect(&config.mongodb).await?;
                    cx.spawn_child(ApiWorker::new(db)).await;
                }
                ActorError::Panic | ActorError::Aborted => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    std::panic::set_hook(Box::new(|p| {
        log::error!("{}", p);
    }));

    if let Err(e) = Runtime::launch(startup).await {
        log::error!("{}", e);
    }
}

async fn startup(scope: &mut RuntimeScope) -> Result<(), Box<dyn Error + Send + Sync>> {
    let launcher_addr = scope.spawn_actor_unsupervised(Launcher).await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        launcher_addr.shutdown();
    });

    Ok(())
}
