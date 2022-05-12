// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, HandleEvent, Report, RuntimeError},
};
use clap::Parser;
use thiserror::Error;

use super::{
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

        let db = MongoDb::connect(&config.mongodb).await?;

        if let Some(node_status) = db.status().await? {
            log::info!("{:?}", node_status);
        } else {
            log::info!("No node status has been found in the database, it seems like the database is empty.");
        };

        #[cfg(all(feature = "stardust", feature = "inx"))]
        cx.spawn_child(super::collector::Collector::new(db.clone(), config.collector.clone()))
            .await;

        #[cfg(all(feature = "stardust", feature = "inx"))]
        cx.spawn_child(super::stardust_inx::InxWorker::new(db.clone(), config.inx.clone()))
            .await;

        #[cfg(feature = "api")]
        cx.spawn_child(super::api::ApiWorker::new(db, config.api.clone())).await;

        #[cfg(feature = "metrics")]
        cx.spawn_child(super::metrics::MetricsWorker::new(config.metrics.clone()))
            .await;

        Ok(config)
    }
}

#[cfg(all(feature = "stardust", feature = "inx"))]
#[async_trait]
impl HandleEvent<Report<super::collector::Collector>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<super::collector::Collector>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    super::collector::CollectorError::MongoDb(e) => match e.kind.as_ref() {
                        // Only a few possible errors we could potentially recover from
                        mongodb::error::ErrorKind::Io(_)
                        | mongodb::error::ErrorKind::ServerSelection { message: _, .. } => {
                            let db = MongoDb::connect(&config.mongodb).await?;
                            cx.spawn_child(super::collector::Collector::new(db, config.collector.clone()))
                                .await;
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
impl HandleEvent<Report<super::stardust_inx::InxWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<super::stardust_inx::InxWorker>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match &e.error {
                ActorError::Result(e) => match e {
                    crate::stardust_inx::InxWorkerError::ConnectionError(_) => {
                        let wait_interval = config.inx.connection_retry_interval;
                        log::info!("Retrying INX connection in {} seconds.", wait_interval.as_secs_f32());
                        let db = MongoDb::connect(&config.mongodb).await?;
                        cx.delay(
                            chronicle::runtime::SpawnActor::new(super::stardust_inx::InxWorker::new(
                                db,
                                config.inx.clone(),
                            )),
                            wait_interval,
                        )?;
                    }
                    // TODO: This is stupid, but we can't use the ErrorKind enum so :shrug:
                    crate::stardust_inx::InxWorkerError::TransportFailed(e) => match e.to_string().as_ref() {
                        "transport error" => {
                            let db = MongoDb::connect(&config.mongodb).await?;
                            cx.spawn_child(super::stardust_inx::InxWorker::new(db, config.inx.clone()))
                                .await;
                        }
                        _ => {
                            cx.shutdown();
                        }
                    },
                    crate::stardust_inx::InxWorkerError::MissingCollector => {
                        let db = MongoDb::connect(&config.mongodb).await?;
                        cx.delay(
                            chronicle::runtime::SpawnActor::new(super::stardust_inx::InxWorker::new(
                                db,
                                config.inx.clone(),
                            )),
                            None,
                        )?;
                    }
                    crate::stardust_inx::InxWorkerError::FailedToAnswerRequest
                    | crate::stardust_inx::InxWorkerError::InxTypeConversion(_)
                    | crate::stardust_inx::InxWorkerError::ListenerError(_)
                    | crate::stardust_inx::InxWorkerError::Runtime(_)
                    | crate::stardust_inx::InxWorkerError::Read(_)
                    | crate::stardust_inx::InxWorkerError::ParsingAddressFailed(_)
                    | crate::stardust_inx::InxWorkerError::InvalidAddress(_) => {
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
impl HandleEvent<Report<super::api::ApiWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<super::api::ApiWorker>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let db = MongoDb::connect(&config.mongodb).await?;
                    cx.spawn_child(super::api::ApiWorker::new(db, config.api.clone())).await;
                }
                ActorError::Panic | ActorError::Aborted => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[cfg(feature = "metrics")]
#[async_trait]
impl HandleEvent<Report<super::metrics::MetricsWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<super::metrics::MetricsWorker>,
        config: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    cx.spawn_child(super::metrics::MetricsWorker::new(config.metrics.clone()))
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
