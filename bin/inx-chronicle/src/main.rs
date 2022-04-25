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

use std::{error::Error, ops::Deref};

use async_trait::async_trait;
#[cfg(feature = "stardust")]
use chronicle::{
    db::{MongoDb, MongoDbError},
    runtime::{
        actor::{
            addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, util::SpawnActor,
            Actor,
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
    MongoDb(#[from] MongoDbError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Debug)]
/// Supervisor actor
pub struct Launcher;

#[async_trait]
impl Actor for Launcher {
    type State = (ChronicleConfig, Addr<Broker>);
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
        let broker_addr = cx.spawn_actor_supervised(Broker::new(db.clone())).await;
        #[cfg(feature = "inx")]
        cx.spawn_actor_supervised(InxWorker::new(config.inx.clone(), broker_addr.clone()))
            .await;

        #[cfg(feature = "api")]
        cx.spawn_actor_supervised(ApiWorker::new(db)).await;
        Ok((config, broker_addr))
    }
}

#[async_trait]
impl HandleEvent<Report<Broker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Broker>,
        (config, broker_addr): &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match e.error {
                ActorError::Result(e) => match e.deref() {
                    BrokerError::RuntimeError(_) => {
                        cx.shutdown();
                    }
                    BrokerError::MongoDbError(e) => match e {
                        chronicle::db::MongoDbError::DatabaseError(e) => match e.kind.as_ref() {
                            // Only a few possible errors we could potentially recover from
                            ErrorKind::Io(_) | ErrorKind::ServerSelection { message: _, .. } => {
                                let db = MongoDb::connect(&config.mongodb).await?;
                                let handle = cx.spawn_actor_supervised(Broker::new(db)).await;
                                *broker_addr = handle;
                            }
                            _ => {
                                cx.shutdown();
                            }
                        },
                        other => {
                            log::warn!("Unhandled MongoDB error: {}", other);
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
        (config, broker_addr): &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match &e.error {
                ActorError::Result(e) => match e.deref() {
                    InxWorkerError::ConnectionError(_) => {
                        let wait_interval = config.inx.connection_retry_interval;
                        log::info!("Retrying INX connection in {} seconds.", wait_interval.as_secs_f32());
                        cx.delay(
                            SpawnActor::new(InxWorker::new(config.inx.clone(), broker_addr.clone())),
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
                            cx.spawn_actor_supervised(InxWorker::new(config.inx.clone(), broker_addr.clone()))
                                .await;
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
                        if broker_addr.is_closed() {
                            cx.delay(event, None)?;
                        } else {
                            cx.spawn_actor_supervised(InxWorker::new(config.inx.clone(), broker_addr.clone()))
                                .await;
                        }
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
        (config, _): &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match e.error {
                ActorError::Result(_) => {
                    let db = MongoDb::connect(&config.mongodb).await?;
                    cx.spawn_actor_supervised(ApiWorker::new(db)).await;
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
    let launcher_addr = scope.spawn_actor(Launcher).await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        launcher_addr.shutdown();
    });

    Ok(())
}
