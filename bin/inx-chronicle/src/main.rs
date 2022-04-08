// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

mod broker;
mod cli;
mod config;
mod listener;

use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use broker::Broker;
use chronicle::{
    db::MongoConfig,
    runtime::{
        actor::{context::ActorContext, envelope::HandleEvent, error::ActorError, report::Report, Actor},
        error::RuntimeError,
        scope::RuntimeScope,
        Runtime,
    },
};
use clap::Parser;
use listener::InxListener;
use thiserror::Error;

use self::cli::CliArgs;

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
}

#[derive(Debug)]
/// Supervisor actor
pub struct Launcher;

#[async_trait]
impl Actor for Launcher {
    type Data = ();
    type Error = LauncherError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        cx.spawn_actor_supervised(Broker).await;
        cx.spawn_actor_supervised(InxListener).await;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<Broker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Broker>,
        _data: &mut Self::Data,
    ) -> Result<(), Self::Error> {
        // TODO: Figure out why `cx.shutdown()` is not working.
        let handle = cx.handle();
        match event {
            Ok(_) => {
                handle.shutdown().await;
            }
            Err(e) => match e.error {
                ActorError::Result(_) | ActorError::Panic => {
                    cx.spawn_actor_supervised(Broker).await;
                }
                ActorError::Aborted => {
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
        _data: &mut Self::Data,
    ) -> Result<(), Self::Error> {
        // TODO: Figure out why `cx.shutdown()` is not working.
        let handle = cx.handle();
        match event {
            Ok(_) => {
                handle.shutdown().await;
            }
            Err(e) => match e.error {
                ActorError::Result(_) | ActorError::Panic => {
                    cx.spawn_actor_supervised(InxListener).await;
                }
                ActorError::Aborted => {
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
    std::panic::set_hook(Box::new(|info| {
        log::error!("{}", info);
    }));

    if let Err(e) = Runtime::launch(startup).await {
        log::error!("{}", e);
    }
}

async fn startup(scope: &mut RuntimeScope) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli_args = CliArgs::parse();
    if let Some(config_path) = cli_args.config {
        let config = config::Config::from_file(config_path)?;
        let db = config.mongodb.build().await?;
        scope.add_resource(Arc::new(config)).await;
        scope.add_resource(db).await;
    } else {
        let db = MongoConfig::new("mongodb://localhost:27017".into()).build().await?;
        scope.add_resource(db).await;
    }

    let launcher_handle = scope.spawn_actor(Launcher).await;

    tokio::signal::ctrl_c().await?;
    launcher_handle.shutdown().await;
    Ok(())
}
