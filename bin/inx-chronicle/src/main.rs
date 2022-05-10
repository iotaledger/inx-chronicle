// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
#[cfg(all(feature = "stardust", feature = "inx"))]
mod collector;
mod config;
#[cfg(all(feature = "stardust", feature = "inx"))]
mod stardust_inx;

use std::error::Error;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, HandleEvent, Report, Runtime, RuntimeError, RuntimeScope},
};
use clap::Parser;
use cli::CliArgs;
#[cfg(all(feature = "stardust", feature = "inx"))]
use collector::{Collector, CollectorError};
use config::{ChronicleConfig, ConfigError};
use thiserror::Error;

#[cfg(feature = "api")]
use self::api::ApiWorker;
#[cfg(all(feature = "stardust", feature = "inx"))]
use self::stardust_inx::{StardustInxWorker, StardustInxWorkerError};

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

        #[cfg(all(feature = "stardust", feature = "inx"))]
        cx.spawn_child(Collector::new(db.clone(), config.collector.clone()))
            .await;

        #[cfg(all(feature = "stardust", feature = "inx"))]
        cx.spawn_child(StardustInxWorker::new(config.inx.clone())).await;

        #[cfg(feature = "api")]
        cx.spawn_child(ApiWorker::new(db, config.api.clone())).await;
        Ok(config)
    }
}

#[cfg(all(feature = "stardust", feature = "inx"))]
#[async_trait]
impl HandleEvent<Report<Collector>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Collector>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    CollectorError::MongoDb(e) => match e.kind.as_ref() {
                        // Only a few possible errors we could potentially recover from
                        mongodb::error::ErrorKind::Io(_)
                        | mongodb::error::ErrorKind::ServerSelection { message: _, .. } => {
                            let db = MongoDb::connect(&config.mongodb).await?;
                            cx.spawn_child(Collector::new(db, config.collector.clone())).await;
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

#[cfg(all(feature = "stardust", feature = "inx"))]
#[async_trait]
impl HandleEvent<Report<StardustInxWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<StardustInxWorker>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match &e.error {
                ActorError::Result(e) => match e {
                    StardustInxWorkerError::ConnectionError(_) => {
                        let wait_interval = config.inx.connection_retry_interval;
                        log::info!("Retrying INX connection in {} seconds.", wait_interval.as_secs_f32());
                        cx.delay(
                            chronicle::runtime::SpawnActor::new(StardustInxWorker::new(config.inx.clone())),
                            wait_interval,
                        )?;
                    }
                    StardustInxWorkerError::InvalidAddress(_) => {
                        cx.shutdown();
                    }
                    StardustInxWorkerError::ParsingAddressFailed(_) => {
                        cx.shutdown();
                    }
                    // TODO: This is stupid, but we can't use the ErrorKind enum so :shrug:
                    StardustInxWorkerError::TransportFailed(e) => match e.to_string().as_ref() {
                        "transport error" => {
                            cx.spawn_child(StardustInxWorker::new(config.inx.clone())).await;
                        }
                        _ => {
                            cx.shutdown();
                        }
                    },
                    StardustInxWorkerError::Read(_) => {
                        cx.shutdown();
                    }
                    StardustInxWorkerError::Runtime(_) => {
                        cx.shutdown();
                    }
                    StardustInxWorkerError::ListenerError(_) => {
                        cx.shutdown();
                    }
                    StardustInxWorkerError::MissingCollector => {
                        cx.delay(
                            chronicle::runtime::SpawnActor::new(StardustInxWorker::new(config.inx.clone())),
                            None,
                        )?;
                    }
                    StardustInxWorkerError::FailedToAnswerRequest => {
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
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let db = MongoDb::connect(&config.mongodb).await?;
                    cx.spawn_child(ApiWorker::new(db, config.api.clone())).await;
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
