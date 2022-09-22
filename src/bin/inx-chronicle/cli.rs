// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, Subcommand};

use crate::config::{ChronicleConfig, ConfigError};

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct ClArgs {
    /// The location of the configuration file.
    #[clap(short, long, env = "CONFIG_PATH")]
    pub config: Option<String>,
    /// The MongoDB connection string.
    #[clap(long, env = "MONGODB_CONN_URI")]
    pub mongodb_conn_url: Option<String>,
    /// The address of the INX interface provided by the node.
    #[clap(long, env = "INX_ADDR")]
    #[cfg(feature = "inx")]
    pub inx_addr: Option<String>,
    /// Toggle INX write workflow.
    #[clap(long, value_parser, env = "INX")]
    #[cfg(feature = "inx")]
    pub enable_inx: Option<bool>,
    /// The location of the identity file for JWT auth.
    #[clap(long, env = "JWT_IDENTITY_PATH")]
    #[cfg(feature = "api")]
    pub identity_path: Option<String>,
    /// The password used for JWT authentication.
    #[clap(long)]
    #[cfg(feature = "api")]
    pub password: Option<String>,
    /// Toggle REST API.
    #[clap(long, value_parser, env = "API")]
    #[cfg(feature = "api")]
    pub enable_api: Option<bool>,
    /// Toggle the metrics server.
    #[clap(long, value_parser, env = "METRICS")]
    pub enable_metrics: Option<bool>,
    /// Subcommands.
    #[clap(subcommand)]
    pub subcommand: Option<Subcommands>,
}

impl ClArgs {
    /// Get a config file with CLI args applied.
    pub fn get_config(&self) -> Result<ChronicleConfig, ConfigError> {
        let mut config = self
            .config
            .as_ref()
            .map(ChronicleConfig::from_file)
            .transpose()?
            .unwrap_or_default();

        if let Some(connect_url) = &self.mongodb_conn_url {
            config.mongodb.connect_url = connect_url.clone();
        }

        #[cfg(all(feature = "stardust", feature = "inx"))]
        {
            if let Some(connect_url) = &self.inx_addr {
                config.inx.connect_url = connect_url.clone();
            }
            if let Some(enabled) = self.enable_inx {
                config.inx.enabled = enabled;
            }
        }

        #[cfg(feature = "api")]
        {
            if let Some(password) = &self.password {
                config.api.password_hash = hex::encode(
                    argon2::hash_raw(
                        password.as_bytes(),
                        config.api.password_salt.as_bytes(),
                        &Into::into(&config.api.argon_config),
                    )
                    .expect("invalid JWT config"),
                );
            }
            if let Some(path) = &self.identity_path {
                config.api.identity_path.replace(path.clone());
            }
            if let Some(enabled) = self.enable_api {
                config.api.enabled = enabled;
            }
        }

        if let Some(enabled) = self.enable_metrics {
            config.metrics.enabled = enabled;
        }

        Ok(config)
    }

    /// Process subcommands and return whether the app should early exit.
    #[allow(unused)]
    #[allow(clippy::collapsible_match)]
    pub fn process_subcommands(&self, config: &ChronicleConfig) -> bool {
        if let Some(subcommand) = &self.subcommand {
            match subcommand {
                #[cfg(feature = "api")]
                Subcommands::GenerateJWT => {
                    use crate::api::ApiData;
                    let api_data = ApiData::try_from(config.api.clone()).expect("invalid API config");
                    let jwt = auth_helper::jwt::JsonWebToken::new(
                        auth_helper::jwt::Claims::new(
                            ApiData::ISSUER,
                            uuid::Uuid::new_v4().to_string(),
                            ApiData::AUDIENCE,
                        )
                        .unwrap()
                        .expires_after_duration(api_data.jwt_expiration)
                        .expect("invalid JWT config"),
                        api_data.secret_key.as_ref(),
                    )
                    .expect("invalid JWT config");
                    println!("Bearer {}", jwt);
                    return true;
                }
                _ => (),
            }
        }
        false
    }
}

#[derive(Debug, Subcommand)]
pub enum Subcommands {
    /// Generate a JWT token using the available config.
    #[cfg(feature = "api")]
    GenerateJWT,
}
