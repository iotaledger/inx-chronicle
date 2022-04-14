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
    db::MongoDbError,
    inx::InxError,
    runtime::{
        actor::{
            addr::{Addr, SendError},
            context::ActorContext,
            error::ActorError,
            event::HandleEvent,
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
    type State = (Config, Addr<Broker>);
    type Error = LauncherError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let cli_args = CliArgs::parse();
        let config = match cli_args.config_path {
            Some(path) => config::Config::from_file(path)?,
            None => Config::default(),
        };
        let db = config.mongodb.clone().build().await?;
        let broker_addr = cx.spawn_actor_supervised(Broker::new(db)).await;
        cx.spawn_actor_supervised(InxListener::new(config.inx.clone(), broker_addr.clone()))
            .await;
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
                ActorError::Result(e) => match e.downcast_ref::<BrokerError>().unwrap() {
                    BrokerError::RuntimeError(_) => {
                        cx.shutdown();
                    }
                    BrokerError::MongoDbError(e) => match e {
                        chronicle::db::MongoDbError::DatabaseError(e) => match e.kind.as_ref() {
                            // Only a few possible errors we could potentially recover from
                            ErrorKind::Io(_) | ErrorKind::ServerSelection { message: _, .. } => {
                                let db = config.mongodb.clone().build().await?;
                                let handle = cx.spawn_actor_supervised(Broker::new(db)).await;
                                *broker_addr = handle;
                            }
                            _ => {
                                cx.shutdown();
                            }
                        },
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

#[async_trait]
impl HandleEvent<Report<InxListener>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxListener>,
        (config, broker_addr): &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match &e.error {
                ActorError::Result(e) => match e.downcast_ref::<InxListenerError>().unwrap() {
                    InxListenerError::Inx(e) => match e {
                        InxError::TransportFailed => {
                            cx.spawn_actor_supervised(InxListener::new(config.inx.clone(), broker_addr.clone()))
                                .await;
                        }
                    },
                    InxListenerError::Read(_) => {
                        cx.shutdown();
                    }
                    InxListenerError::Runtime(_) => {
                        cx.shutdown();
                    }
                    InxListenerError::MissingBroker => {
                        // If the handle is still closed, push this to the back of the event queue.
                        // Hopefully when it is processed again the handle will have been recreated.
                        if broker_addr.is_closed() {
                            cx.handle().send(event)?;
                        } else {
                            cx.spawn_actor_supervised(InxListener::new(config.inx.clone(), broker_addr.clone()))
                                .await;
                        }
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

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

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
