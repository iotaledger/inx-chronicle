// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, Subcommand};

use crate::{
    config::{ChronicleConfig, ConfigError},
    error::Error,
};

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct ClArgs {
    /// The location of the configuration file.
    #[arg(short, long, env = "CONFIG_PATH")]
    pub config: Option<String>,
    /// The MongoDB connection string.
    #[arg(long = "mongodb.conn-str", env = "MONGODB_CONN_STR")]
    pub mongodb_conn_str: Option<String>,
    /// The url pointing to an InfluxDb instance.
    #[arg(long = "influxdb.url", env = "INFLUXDB_URL")]
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub influxdb_url: Option<String>,
    /// The address of the INX interface provided by the node.
    #[arg(long = "inx.url", env = "INX_URL")]
    #[cfg(feature = "inx")]
    pub inx_url: Option<String>,
    /// Toggle INX write workflow.
    #[arg(long = "inx.enabled", env = "INX_ENABLED")]
    #[cfg(feature = "inx")]
    pub enable_inx: Option<bool>,
    /// The location of the identity file for JWT auth.
    #[arg(long = "rest-api.jwt.identity", env = "JWT_IDENTITY_PATH")]
    #[cfg(feature = "api")]
    pub identity_path: Option<String>,
    /// The password used for JWT authentication.
    #[arg(long = "rest-api.jwt.password")]
    #[cfg(feature = "api")]
    pub password: Option<String>,
    /// Toggle REST API.
    #[arg(long = "rest-api.enabled", env = "REST_API_ENABLED")]
    #[cfg(feature = "api")]
    pub enable_api: Option<bool>,
    /// Toggle the metrics server.
    #[arg(long = "prometheus.enabled", env = "PROMETHEUS_ENABLED")]
    pub enable_metrics: Option<bool>,
    /// Subcommands.
    #[command(subcommand)]
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

        if let Some(conn_str) = &self.mongodb_conn_str {
            config.mongodb.conn_str = conn_str.clone();
        }

        #[cfg(all(feature = "stardust", feature = "inx"))]
        {
            if let Some(connect_url) = &self.inx_url {
                config.inx.connect_url = connect_url.clone();
            }
            if let Some(enabled) = self.enable_inx {
                config.inx.enabled = enabled;
            }
            if let Some(url) = &self.influxdb_url {
                config.influxdb.url = url.clone();
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
                    // TODO: Replace this once we switch to a better error lib
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
    pub fn process_subcommands(&self, config: &ChronicleConfig) -> Result<bool, Error> {
        if let Some(subcommand) = &self.subcommand {
            match subcommand {
                #[cfg(feature = "api")]
                Subcommands::GenerateJWT => {
                    use crate::api::ApiData;
                    let api_data = ApiData::try_from(config.api.clone()).expect("invalid API config");
                    let claims = auth_helper::jwt::Claims::new(
                        ApiData::ISSUER,
                        uuid::Uuid::new_v4().to_string(),
                        ApiData::AUDIENCE,
                    )
                    .unwrap() // Panic: Cannot fail.
                    .expires_after_duration(api_data.jwt_expiration)
                    .map_err(crate::api::ApiError::InvalidJwt)?;
                    let exp_ts = time::OffsetDateTime::from_unix_timestamp(claims.exp.unwrap() as _).unwrap();
                    let jwt = auth_helper::jwt::JsonWebToken::new(claims, api_data.secret_key.as_ref())
                        .map_err(crate::api::ApiError::InvalidJwt)?;
                    println!("Bearer {}", jwt);
                    println!(
                        "Expires: {} ({})",
                        exp_ts,
                        humantime::format_duration(api_data.jwt_expiration)
                    );
                    return Ok(true);
                }
                _ => (),
            }
        }
        Ok(false)
    }
}

#[derive(Debug, Subcommand)]
pub enum Subcommands {
    /// Generate a JWT token using the available config.
    #[cfg(feature = "api")]
    GenerateJWT,
}
