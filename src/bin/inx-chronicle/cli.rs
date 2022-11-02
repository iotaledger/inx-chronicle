// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::{ArgGroup, Args, Parser, Subcommand};

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
    /// Rest API arguments.
    #[cfg(feature = "api")]
    #[command(flatten)]
    pub api: ApiArgs,
    /// InfluxDb arguments.
    #[cfg(feature = "influxdb")]
    #[command(flatten)]
    pub influxdb: InfluxDbArgs,
    /// INX arguments.
    #[cfg(feature = "inx")]
    #[command(flatten)]
    pub inx: InxArgs,
    /// Metrics arguments.
    #[command(flatten)]
    pub metrics: MetricsArgs,
    /// MongoDb arguments.
    #[command(flatten)]
    pub mongodb: MongoDbArgs,
    /// Subcommands.
    #[command(subcommand)]
    pub subcommand: Option<Subcommands>,
}

#[cfg(feature = "api")]
#[derive(Args, Debug)]
#[command(group = ArgGroup::new("api").args(["api_enabled"]))]
pub struct ApiArgs {
    /// Toggle REST API.
    #[arg(long, env = "REST_API_ENABLED")]
    pub api_enabled: Option<bool>,
    /// JWT arguments.
    #[command(flatten)]
    pub jwt: JwtArgs,
}

#[derive(Args, Debug)]
#[command(group = ArgGroup::new("jwt").args(["identity_path", "password"]))]
pub struct JwtArgs {
    /// The location of the identity file for JWT auth.
    #[arg(long = "api-jwt-identity", env = "JWT_IDENTITY_PATH")]
    pub identity_path: Option<String>,
    /// The password used for JWT authentication.
    #[arg(long = "api-jwt-password")]
    pub password: Option<String>,
}

#[cfg(feature = "inx")]
#[derive(Args, Debug)]
#[command(group = ArgGroup::new("inx").args(["inx_enabled", "inx_url", "sync_start"]))]
pub struct InxArgs {
    /// Toggle INX write workflow.
    #[arg(long, env = "INX_ENABLED")]
    pub inx_enabled: Option<bool>,
    /// The address of the INX interface provided by the node.
    #[arg(long, env = "INX_URL")]
    pub inx_url: Option<String>,
    /// Set the milestone index at which synchronization should start (1 includes everything until genesis).
    #[arg(long = "inx-sync-start", env = "SYNC_START")]
    pub sync_start: Option<u32>,
}

#[derive(Args, Debug)]
#[command(group = ArgGroup::new("mongodb").args(["mongodb_conn_str"]))]
pub struct MongoDbArgs {
    /// The MongoDB connection string.
    #[arg(long, env = "MONGODB_CONN_STR")]
    pub mongodb_conn_str: Option<String>,
}

#[cfg(feature = "influxdb")]
#[derive(Args, Debug)]
#[command(group = ArgGroup::new("influxdb").args(["influxdb_url"]))]
pub struct InfluxDbArgs {
    /// The url pointing to an InfluxDb instance.
    #[arg(long, env = "INFLUXDB_URL")]
    pub influxdb_url: Option<String>,
}

#[derive(Args, Debug)]
#[command(group = ArgGroup::new("metrics").args(["prometheus_enabled"]))]
pub struct MetricsArgs {
    /// Toggle the prometheus server.
    #[arg(long, env = "PROMETHEUS_ENABLED")]
    pub prometheus_enabled: Option<bool>,
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

        if let Some(conn_str) = &self.mongodb.mongodb_conn_str {
            config.mongodb.conn_str = conn_str.clone();
        }

        #[cfg(all(feature = "stardust", feature = "inx"))]
        {
            if let Some(connect_url) = &self.inx.inx_url {
                config.inx.connect_url = connect_url.clone();
            }
            if let Some(enabled) = self.inx.inx_enabled {
                config.inx.enabled = enabled;
            }
            if let Some(sync_start) = self.inx.sync_start {
                config.inx.sync_start_milestone = sync_start.into();
            }
        }

        #[cfg(feature = "influxdb")]
        if let Some(url) = &self.influxdb.influxdb_url {
            config.influxdb.url = url.clone();
        }

        #[cfg(feature = "api")]
        {
            if let Some(password) = &self.api.jwt.password {
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
            if let Some(path) = &self.api.jwt.identity_path {
                config.api.identity_path.replace(path.clone());
            }
            if let Some(enabled) = self.api.api_enabled {
                config.api.enabled = enabled;
            }
        }

        if let Some(enabled) = self.metrics.prometheus_enabled {
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
