// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use bytesize::ByteSize;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ErrorLevel, RuntimeError},
};
use clap::Parser;
use thiserror::Error;
use tracing::{debug, info};

use super::{
    cli::ClArgs,
    config::{ChronicleConfig, ConfigError},
};

const ENV_CONFIG_PATH: &str = "CONFIG_PATH";

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

impl ErrorLevel for LauncherError {
    fn level(&self) -> tracing::Level {
        match self {
            LauncherError::Config(_) | LauncherError::MongoDb(_) => tracing::Level::ERROR,
            LauncherError::Runtime(_) => tracing::Level::WARN,
        }
    }
}

#[derive(Debug)]
/// Supervisor actor
pub struct Launcher;

#[async_trait]
impl Actor for Launcher {
    type State = ChronicleConfig;
    type Error = LauncherError;

    #[allow(unused_variables)]
    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        // TODO: move parsing command-line args to `main.rs` (only async closures make this low effort though)
        let cl_args = ClArgs::parse();

        let mut config = match &cl_args.config {
            Some(path) => ChronicleConfig::from_file(path)?,
            None => {
                if let Ok(path) = std::env::var(ENV_CONFIG_PATH) {
                    ChronicleConfig::from_file(path)?
                } else {
                    ChronicleConfig::default()
                }
            }
        };
        config.apply_cl_args(&cl_args);

        info!(
            "Connecting to database at bind address `{}`.",
            config.mongodb.connect_url
        );
        let db = MongoDb::connect(&config.mongodb).await?;
        debug!("Available databases: `{:?}`", db.get_databases().await?);
        info!(
            "Connected to database `{}` ({})",
            db.name(),
            ByteSize::b(db.size().await?)
        );

        #[cfg(feature = "stardust")]
        {
            db.create_output_indexes().await?;
            db.create_block_indexes().await?;
            db.create_ledger_update_indexes().await?;
            db.create_milestone_indexes().await?;
        }

        #[cfg(all(feature = "inx", feature = "stardust"))]
        if config.inx.enabled {
            cx.spawn_child(super::stardust_inx::InxWorker::new(&db, &config.inx))
                .await;
        }

        #[cfg(feature = "api")]
        if config.api.enabled {
            cx.spawn_child(super::api::ApiWorker::new(&db, &config.api).map_err(ConfigError::Api)?)
                .await;
        }

        #[cfg(feature = "metrics")]
        if config.metrics.enabled {
            cx.spawn_child(super::metrics::MetricsWorker::new(&db, &config.metrics))
                .await;
        }

        Ok(config)
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Launcher".into()
    }
}

#[cfg(all(feature = "inx", feature = "stardust"))]
#[async_trait]
impl chronicle::runtime::HandleEvent<chronicle::runtime::Report<super::stardust_inx::InxWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: chronicle::runtime::Report<super::stardust_inx::InxWorker>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        use chronicle::runtime::{ActorError, Report};

        use super::stardust_inx::InxError;
        match event {
            Report::Success(_) => {
                cx.abort().await;
            }
            Report::Error(report) => match report.error {
                ActorError::Result(e) => match e {
                    InxError::MongoDb(e) => match e.kind.as_ref() {
                        // Only a few possible errors we could potentially recover from
                        mongodb::error::ErrorKind::Io(_)
                        | mongodb::error::ErrorKind::ServerSelection { message: _, .. } => {
                            let db = MongoDb::connect(&config.mongodb).await?;
                            cx.spawn_child(super::stardust_inx::InxWorker::new(&db, &config.inx))
                                .await;
                        }
                        _ => {
                            cx.abort().await;
                        }
                    },
                    InxError::BeeInx(bee_inx::Error::StatusCode(e)) => match e.code() {
                        tonic::Code::DeadlineExceeded
                        | tonic::Code::ResourceExhausted
                        | tonic::Code::Aborted
                        | tonic::Code::Unavailable => {
                            cx.spawn_child(report.actor).await;
                        }
                        _ => {
                            cx.abort().await;
                        }
                    },
                    _ => {
                        cx.abort().await;
                    }
                },
                ActorError::Panic | ActorError::Aborted => {
                    cx.abort().await;
                }
            },
        }
        Ok(())
    }
}

#[cfg(feature = "api")]
#[async_trait]
impl chronicle::runtime::HandleEvent<chronicle::runtime::Report<super::api::ApiWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: chronicle::runtime::Report<super::api::ApiWorker>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        use chronicle::runtime::{ActorError, Report};
        match event {
            Report::Success(_) => {
                cx.abort().await;
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let db = MongoDb::connect(&config.mongodb).await?;
                    cx.spawn_child(super::api::ApiWorker::new(&db, &config.api).map_err(ConfigError::Api)?)
                        .await;
                }
                ActorError::Panic | ActorError::Aborted => {
                    cx.abort().await;
                }
            },
        }
        Ok(())
    }
}

#[cfg(feature = "metrics")]
#[async_trait]
impl chronicle::runtime::HandleEvent<chronicle::runtime::Report<super::metrics::MetricsWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: chronicle::runtime::Report<super::metrics::MetricsWorker>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        use chronicle::runtime::{ActorError, Report};
        match event {
            Report::Success(_) => {
                cx.abort().await;
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let db = MongoDb::connect(&config.mongodb).await?;
                    cx.spawn_child(super::metrics::MetricsWorker::new(&db, &config.metrics))
                        .await;
                }
                ActorError::Panic | ActorError::Aborted => {
                    cx.abort().await;
                }
            },
        }

        Ok(())
    }
}
