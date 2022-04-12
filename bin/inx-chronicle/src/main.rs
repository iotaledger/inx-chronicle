// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

mod broker;
mod cli;
mod config;
mod listener;

use std::error::Error;

use async_trait::async_trait;
use broker::{Broker, BrokerError};
use chronicle::{
    db::{MongoConfig, MongoDbError},
    inx::{InxConfig, InxError},
    runtime::{
        actor::{
            context::ActorContext,
            envelope::HandleEvent,
            error::ActorError,
            handle::{Addr, SendError},
            report::Report,
            Actor,
        },
        error::RuntimeError,
        scope::RuntimeScope,
        Runtime,
    },
};
use clap::Parser;
use config::Config;
use listener::{InxListener, InxListenerError};
use mongodb::error::ErrorKind;
use thiserror::Error;

use self::cli::CliArgs;

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error(transparent)]
    Send(#[from] SendError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
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
    type Data = (Config, Addr<Broker>);
    type Error = LauncherError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error> {
        let cli_args = CliArgs::parse();
        let config = match cli_args.config {
            Some(path) => config::Config::from_file(path)?,
            None => Config {
                mongodb: MongoConfig::new("mongodb://localhost:27017"),
                inx: InxConfig::new("http://localhost:9029"),
            },
        };
        let db = config.mongodb.clone().build().await?;
        let broker_handle = cx.spawn_actor_supervised(Broker::new(db)).await;
        cx.spawn_actor_supervised(InxListener::new(config.inx.clone(), broker_handle.clone()))
            .await;
        Ok((config, broker_handle))
    }
}

#[async_trait]
impl HandleEvent<Report<Broker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Broker>,
        (config, broker_handle): &mut Self::Data,
    ) -> Result<(), Self::Error> {
        // TODO: Figure out why `cx.shutdown()` is not working.
        let handle = cx.handle();
        match event {
            Ok(_) => {
                handle.shutdown().await;
            }
            Err(e) => match e.error {
                ActorError::Result(e) => match e.downcast_ref::<BrokerError>().unwrap() {
                    BrokerError::RuntimeError(_) => {
                        handle.shutdown().await;
                    }
                    BrokerError::MongoDbError(e) => match e {
                        chronicle::db::MongoDbError::DatabaseError(e) => match e.kind.as_ref() {
                            // Only a few possible errors we could potentially recover from
                            ErrorKind::Io(_) | ErrorKind::ServerSelection { message: _, .. } => {
                                let db = config.mongodb.clone().build().await?;
                                let handle = cx.spawn_actor_supervised(Broker::new(db)).await;
                                *broker_handle = handle;
                            }
                            _ => {
                                handle.shutdown().await;
                            }
                        },
                    },
                },
                ActorError::Panic | ActorError::Aborted => {
                    handle.shutdown().await;
                }
            },
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<InxListener>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxListener>,
        (config, broker_handle): &mut Self::Data,
    ) -> Result<(), Self::Error> {
        // TODO: Figure out why `cx.shutdown()` is not working.
        let handle = cx.handle();
        match &event {
            Ok(_) => {
                handle.shutdown().await;
            }
            Err(e) => match &e.error {
                ActorError::Result(e) => match e.downcast_ref::<InxListenerError>().unwrap() {
                    InxListenerError::Inx(e) => match e {
                        InxError::TransportFailed => {
                            cx.spawn_actor_supervised(InxListener::new(config.inx.clone(), broker_handle.clone()))
                                .await;
                        }
                    },
                    InxListenerError::Read(_) => {
                        handle.shutdown().await;
                    }
                    InxListenerError::Runtime(_) => {
                        handle.shutdown().await;
                    }
                    InxListenerError::MissingBroker => {
                        // If the handle is still closed, push this to the back of the event queue.
                        // Hopefully when it is processed again the handle will have been recreated.
                        if broker_handle.is_closed() {
                            handle.send(event)?;
                        } else {
                            cx.spawn_actor_supervised(InxListener::new(config.inx.clone(), broker_handle.clone()))
                                .await;
                        }
                    }
                },
                ActorError::Panic | ActorError::Aborted => {
                    handle.shutdown().await;
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

    if let Err(e) = Runtime::launch(startup).await {
        log::error!("{}", e);
    }
}

async fn startup(scope: &mut RuntimeScope) -> Result<(), Box<dyn Error + Send + Sync>> {
    let launcher_handle = scope.spawn_actor(Launcher).await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        launcher_handle.shutdown().await;
    });

    Ok(())
}
