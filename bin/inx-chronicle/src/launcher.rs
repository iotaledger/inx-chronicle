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

pub struct LauncherState {
    config: ChronicleConfig,
    #[cfg(feature = "api")]
    secret_key: libp2p_core::identity::ed25519::SecretKey,
}

#[async_trait]
impl Actor for Launcher {
    type State = LauncherState;
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
        config.apply_cli_args(&cli_args);

        let db = MongoDb::connect(&config.mongodb).await?;

        if let Some(node_status) = db.status().await? {
            log::info!("{:?}", node_status);
        } else {
            log::info!("No node status has been found in the database, it seems like the database is empty.");
        };

        #[cfg(all(feature = "inx", feature = "stardust"))]
        cx.spawn_child(super::stardust_inx::Inx::new(db.clone(), config.inx.clone()))
            .await;

        #[cfg(feature = "api")]
        let secret_key = {
            let secret_key = match &cli_args.identity {
                Some(path) => keypair_from_file(path)?,
                None => {
                    if let Ok(path) = std::env::var("IDENTITY_PATH") {
                        keypair_from_file(&path)?
                    } else {
                        libp2p_core::identity::ed25519::SecretKey::generate()
                    }
                }
            };
            cx.spawn_child(super::api::ApiWorker::new(
                db,
                (config.api.clone(), secret_key.clone())
                    .try_into()
                    .map_err(ConfigError::Api)?,
            ))
            .await;
            secret_key
        };

        #[cfg(feature = "metrics")]
        cx.spawn_child(super::metrics::MetricsWorker::new(config.metrics.clone()))
            .await;

        Ok(LauncherState {
            config,
            #[cfg(feature = "api")]
            secret_key,
        })
    }
}

#[cfg(feature = "api")]
fn keypair_from_file(path: &str) -> Result<libp2p_core::identity::ed25519::SecretKey, ConfigError> {
    use ed25519::pkcs8::DecodePrivateKey;
    let mut bytes = ed25519::pkcs8::KeypairBytes::from_pkcs8_pem(
        &std::fs::read_to_string(std::path::Path::new(path)).map_err(ConfigError::FileRead)?,
    )
    .map_err(|_| ConfigError::KeyRead)?;
    libp2p_core::identity::ed25519::SecretKey::from_bytes(&mut bytes.secret_key).map_err(ConfigError::KeyDecode)
}

#[cfg(all(feature = "inx", feature = "stardust"))]
#[async_trait]
impl HandleEvent<Report<super::stardust_inx::Inx>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<super::stardust_inx::Inx>,
        state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.abort().await;
            }
            Report::Error(report) => match report.error {
                ActorError::Result(e) => match e {
                    super::stardust_inx::InxError::MongoDb(e) => match e.kind.as_ref() {
                        // Only a few possible errors we could potentially recover from
                        mongodb::error::ErrorKind::Io(_)
                        | mongodb::error::ErrorKind::ServerSelection { message: _, .. } => {
                            let db = MongoDb::connect(&state.config.mongodb).await?;
                            cx.spawn_child(super::stardust_inx::Inx::new(db, state.config.inx.clone()))
                                .await;
                        }
                        _ => {
                            cx.abort().await;
                        }
                    },
                    super::stardust_inx::InxError::Read(_) | super::stardust_inx::InxError::TransportFailed(_) => {
                        cx.spawn_child(report.actor).await;
                    }
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
impl HandleEvent<Report<super::api::ApiWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<super::api::ApiWorker>,
        state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.abort().await;
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let db = MongoDb::connect(&state.config.mongodb).await?;
                    cx.spawn_child(super::api::ApiWorker::new(
                        db,
                        (state.config.api.clone(), state.secret_key.clone())
                            .try_into()
                            .map_err(ConfigError::Api)?,
                    ))
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
impl HandleEvent<Report<super::metrics::MetricsWorker>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<super::metrics::MetricsWorker>,
        state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.abort().await;
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    cx.spawn_child(super::metrics::MetricsWorker::new(state.config.metrics.clone()))
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
