// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use api::ApiConfig;
use clap::{Args, Parser};

use crate::api::config as api;

#[derive(Args, Debug)]
pub struct ApiArgs {
    /// API listening port.
    #[arg(long, value_name = "PORT", default_value_t = api::DEFAULT_PORT)]
    pub api_port: u16,
    /// CORS setting.
    #[arg(long = "allow-origin", value_name = "IP", default_value = api::DEFAULT_ALLOW_ORIGINS)]
    pub allow_origins: Vec<String>,
    /// Public API routes.
    #[arg(long = "public-route", value_name = "ROUTE", default_value = api::DEFAULT_PUBLIC_ROUTES)]
    pub public_routes: Vec<String>,
    /// Maximum number of results returned by a single API call.
    #[arg(long, value_name = "SIZE", default_value_t = api::DEFAULT_MAX_PAGE_SIZE)]
    pub max_page_size: usize,
    /// JWT arguments.
    #[command(flatten)]
    pub jwt: JwtArgs,
    /// Disable REST API.
    #[arg(long, default_value_t = !api::DEFAULT_ENABLED)]
    pub disable_api: bool,
}

impl From<&ApiArgs> for api::ApiConfig {
    fn from(value: &ApiArgs) -> Self {
        Self {
            enabled: !value.disable_api,
            port: value.api_port,
            allow_origins: (&value.allow_origins).into(),
            jwt_password: value.jwt.jwt_password.clone(),
            jwt_salt: value.jwt.jwt_salt.clone(),
            jwt_identity_file: value.jwt.jwt_identity.clone(),
            jwt_expiration: value.jwt.jwt_expiration,
            max_page_size: value.max_page_size,
            public_routes: value.public_routes.clone(),
        }
    }
}

#[derive(Args, Debug)]
pub struct JwtArgs {
    /// The location of the identity file for JWT auth.
    #[arg(long, value_name = "FILEPATH", env = "JWT_IDENTITY", default_value = None)]
    pub jwt_identity: Option<String>,
    /// The password used for JWT authentication.
    #[arg(long, value_name = "PASSWORD", env = "JWT_PASSWORD", default_value = api::DEFAULT_JWT_PASSWORD)]
    pub jwt_password: String,
    /// The salt used for JWT authentication.
    #[arg(long, value_name = "SALT", env = "JWT_SALT", default_value = api::DEFAULT_JWT_SALT)]
    pub jwt_salt: String,
    /// The setting for when the (JWT) token expires.
    #[arg(long, value_name = "DURATION", value_parser = parse_duration, default_value = api::DEFAULT_JWT_EXPIRATION)]
    pub jwt_expiration: std::time::Duration,
}

fn parse_duration(arg: &str) -> Result<std::time::Duration, humantime::DurationError> {
    arg.parse::<humantime::Duration>().map(Into::into)
}

/// Generate a JWT token using the available config.
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
pub struct GenerateJWTCommand;

impl GenerateJWTCommand {
    pub fn handle(&self, config: &ApiConfig) -> eyre::Result<()> {
        use crate::api::ApiConfigData;
        let api_data = ApiConfigData::try_from(config.clone()).expect("invalid API config");
        let claims = auth_helper::jwt::Claims::new(
            ApiConfigData::ISSUER,
            uuid::Uuid::new_v4().to_string(),
            ApiConfigData::AUDIENCE,
        )
        .unwrap() // Panic: Cannot fail.
        .expires_after_duration(api_data.jwt_expiration)
        .map_err(crate::api::AuthError::InvalidJwt)?;
        let exp_ts = time::OffsetDateTime::from_unix_timestamp(claims.exp.unwrap() as _).unwrap();
        let jwt = auth_helper::jwt::JsonWebToken::new(claims, api_data.jwt_secret_key.as_ref())
            .map_err(crate::api::AuthError::InvalidJwt)?;
        tracing::info!("Bearer {}", jwt);
        tracing::info!(
            "Expires: {} ({})",
            exp_ts,
            humantime::format_duration(api_data.jwt_expiration)
        );
        Ok(())
    }
}
