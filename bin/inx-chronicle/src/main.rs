// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
pub mod api;
mod archiver;
mod broker;
mod cli;
mod collector;
mod config;
#[cfg(feature = "stardust")]
mod inx;
mod util;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    error::Error,
    fmt::Debug,
    time::Duration,
};

#[cfg(feature = "api")]
use api::ApiWorker;
use archiver::Archiver;
use async_trait::async_trait;
use broker::{Broker, BrokerError};
#[cfg(feature = "stardust")]
use chronicle::{
    db::MongoDbError,
    inx::InxError,
    runtime::{
        actor::{
            addr::{Addr, OptionalAddr},
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
use collector::{Collector, CollectorError};
use config::{Config, ConfigError};
use mongodb::error::ErrorKind;
use thiserror::Error;
use tokio::sync::RwLock;
use util::SpawnRegistryActor;

use self::cli::CliArgs;
#[cfg(feature = "stardust")]
use self::inx::{InxWorker, InxWorkerError};

lazy_static::lazy_static! {
    /// This is here because it's nice to have a central registry, but also because
    /// of circular dependencies.
    static ref ADDRESS_REGISTRY: AddressMap = Default::default();
}

#[derive(Debug, Default)]
pub struct AddressMap {
    map: RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl AddressMap {
    pub async fn insert<T>(&self, addr: Addr<T>)
    where
        T: Actor + Send + Sync + 'static,
    {
        self.map.write().await.insert(TypeId::of::<T>(), Box::new(addr));
    }

    pub async fn get<T>(&self) -> OptionalAddr<T>
    where
        T: Actor + Send + Sync + 'static,
    {
        self.map
            .read()
            .await
            .get(&TypeId::of::<T>())
            .and_then(|addr| addr.downcast_ref())
            .and_then(|addr: &Addr<T>| (!addr.is_closed()).then(|| addr.clone()))
            .into()
    }
}

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
pub struct Launcher {
    inx_connection_retry_interval: Duration,
}

#[async_trait]
impl Actor for Launcher {
    type State = Config;
    type Error = LauncherError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let cli_args = CliArgs::parse();
        let mut config = match &cli_args.config {
            Some(path) => config::Config::from_file(path)?,
            None => {
                if let Ok(path) = std::env::var("CONFIG_PATH") {
                    config::Config::from_file(path)?
                } else {
                    Config::default()
                }
            }
        };
        config.apply_cli_args(cli_args);

        let db = config.mongodb.clone().build().await?;
        ADDRESS_REGISTRY.insert(cx.spawn_actor_supervised(Archiver).await).await;
        ADDRESS_REGISTRY
            .insert(cx.spawn_actor_supervised(Collector::new(db.clone(), 1)).await)
            .await;
        ADDRESS_REGISTRY
            .insert(cx.spawn_actor_supervised(Broker::new(db.clone())).await)
            .await;

        #[cfg(feature = "stardust")]
        {
            ADDRESS_REGISTRY
                .insert(cx.spawn_actor_supervised(InxWorker::new(config.inx.clone())).await)
                .await;
        }

        #[cfg(feature = "api")]
        ADDRESS_REGISTRY
            .insert(cx.spawn_actor_supervised(ApiWorker::new(db)).await)
            .await;

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
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    BrokerError::MissingCollector => {
                        if ADDRESS_REGISTRY.get::<Collector>().await.is_none() {
                            cx.delay(<Report<Broker>>::Error(report), None)?;
                        } else {
                            ADDRESS_REGISTRY
                                .insert(cx.spawn_actor_supervised(report.actor).await)
                                .await;
                        }
                    }
                    BrokerError::RuntimeError(_) => {
                        cx.shutdown();
                    }
                    BrokerError::MongoDbError(e) => match e {
                        chronicle::db::MongoDbError::DatabaseError(e) => match e.kind.as_ref() {
                            // Only a few possible errors we could potentially recover from
                            ErrorKind::Io(_) | ErrorKind::ServerSelection { message: _, .. } => {
                                let db = config.mongodb.clone().build().await?;
                                ADDRESS_REGISTRY
                                    .insert(cx.spawn_actor_supervised(Broker::new(db)).await)
                                    .await;
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
                    CollectorError::MongoDb(e) => match e {
                        chronicle::db::MongoDbError::DatabaseError(e) => match e.kind.as_ref() {
                            // Only a few possible errors we could potentially recover from
                            ErrorKind::Io(_) | ErrorKind::ServerSelection { message: _, .. } => {
                                let db = config.mongodb.clone().build().await?;
                                ADDRESS_REGISTRY
                                    .insert(cx.spawn_actor_supervised(Collector::new(db, 1)).await)
                                    .await;
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

#[async_trait]
impl HandleEvent<Report<Archiver>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Archiver>,
        _config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                #[allow(clippy::match_single_binding)]
                ActorError::Result(e) => match e {
                    // TODO
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

#[cfg(feature = "stardust")]
mod stardust {
    use super::*;

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
                Report::Error(e) => match &e.error {
                    ActorError::Result(e) => match e {
                        InxWorkerError::Inx(e) => match e {
                            InxError::ConnectionError(_) => {
                                let wait_interval = self.inx_connection_retry_interval;
                                log::info!("Retrying INX connection in {} seconds.", wait_interval.as_secs_f32());
                                cx.delay(
                                    SpawnRegistryActor::new(InxWorker::new(config.inx.clone())),
                                    wait_interval,
                                )?;
                            }
                            InxError::InvalidAddress(_) => {
                                cx.shutdown();
                            }
                            InxError::ParsingAddressFailed(_) => {
                                cx.shutdown();
                            }
                            // TODO: This is stupid, but we can't use the ErrorKind enum so :shrug:
                            InxError::TransportFailed(e) => match e.to_string().as_ref() {
                                "transport error" => {
                                    ADDRESS_REGISTRY
                                        .insert(cx.spawn_actor_supervised(InxWorker::new(config.inx.clone())).await)
                                        .await;
                                }
                                _ => {
                                    cx.shutdown();
                                }
                            },
                        },
                        InxWorkerError::Read(_) => {
                            cx.shutdown();
                        }
                        InxWorkerError::Runtime(_) => {
                            cx.shutdown();
                        }
                        InxWorkerError::MissingBroker => {
                            // If the handle is still closed, push this to the back of the event queue.
                            // Hopefully when it is processed again the handle will have been recreated.
                            if ADDRESS_REGISTRY.get::<Broker>().await.is_none() {
                                cx.delay(event, None)?;
                            } else {
                                ADDRESS_REGISTRY
                                    .insert(cx.spawn_actor_supervised(InxWorker::new(config.inx.clone())).await)
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
                    let db = config.mongodb.clone().build().await?;
                    ADDRESS_REGISTRY
                        .insert(cx.spawn_actor_supervised(ApiWorker::new(db)).await)
                        .await;
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
impl<A> HandleEvent<SpawnRegistryActor<A>> for Launcher
where
    Launcher: HandleEvent<Report<A>>,
    A: 'static + Actor + Debug + Send + Sync,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: SpawnRegistryActor<A>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        ADDRESS_REGISTRY
            .insert(cx.spawn_actor_supervised(event.actor).await)
            .await;
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
    let launcher = Launcher {
        inx_connection_retry_interval: std::time::Duration::from_secs(5),
    };

    let launcher_addr = scope.spawn_actor(launcher).await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        launcher_addr.shutdown();
    });

    Ok(())
}
