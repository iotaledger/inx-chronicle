// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use derive_more::From;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use tower_http::cors::AllowOrigin;

use super::{error::ConfigError, SecretKey};

/// API configuration
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub enabled: bool,
    pub port: u16,
    pub allow_origins: SingleOrMultiple<String>,
    pub password_hash: String,
    pub password_salt: String,
    #[serde(with = "humantime_serde")]
    pub jwt_expiration: Duration,
    pub public_routes: Vec<String>,
    pub identity_path: Option<String>,
    pub max_page_size: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 8042,
            allow_origins: "*".to_string().into(),
            password_hash: "c42cf2be3a442a29d8cd827a27099b0c".to_string(),
            password_salt: "saltines".to_string(),
            // 72 hours
            jwt_expiration: Duration::from_secs(72 * 60 * 60),
            public_routes: Default::default(),
            identity_path: None,
            max_page_size: 1000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApiData {
    pub port: u16,
    pub allow_origins: AllowOrigin,
    pub password_hash: Vec<u8>,
    pub password_salt: String,
    pub jwt_expiration: Duration,
    pub public_routes: RegexSet,
    pub secret_key: SecretKey,
    pub max_page_size: usize,
}

impl ApiData {
    pub const ISSUER: &'static str = "chronicle";
    pub const AUDIENCE: &'static str = "api";
}

impl TryFrom<ApiConfig> for ApiData {
    type Error = ConfigError;

    fn try_from(config: ApiConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            port: config.port,
            allow_origins: AllowOrigin::try_from(config.allow_origins)?,
            password_hash: hex::decode(config.password_hash)?,
            password_salt: config.password_salt,
            jwt_expiration: config.jwt_expiration,
            public_routes: RegexSet::new(config.public_routes.iter().map(route_to_regex).collect::<Vec<_>>())?,
            secret_key: match &config.identity_path {
                Some(path) => SecretKey::from_file(path)?,
                None => {
                    if let Ok(path) = std::env::var("IDENTITY_PATH") {
                        SecretKey::from_file(&path)?
                    } else {
                        SecretKey::generate()
                    }
                }
            },
            max_page_size: config.max_page_size,
        })
    }
}

fn route_to_regex(route: &impl AsRef<str>) -> String {
    // Escape the string to make sure a regex can be built from it.
    // Existing wildcards `*` get escaped to `\\*`.
    let mut escaped: String = regex::escape(route.as_ref());
    // Convert the escaped wildcard to a valid regex.
    escaped = escaped.replace("\\*", ".*");
    // End the regex.
    escaped.push('$');
    escaped
}

/// Convenience type that allows specifying either a single value or a list of values
/// in the configuration file.
///
/// ## Examples
/// ```toml
/// [api]
/// allow_origins = "origin"
/// allow_origins = ["origin1", "origin2"]
/// ```
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, From)]
#[serde(untagged)]
pub enum SingleOrMultiple<T> {
    Single(T),
    Multiple(Vec<T>),
}

impl TryFrom<SingleOrMultiple<String>> for AllowOrigin {
    type Error = ConfigError;

    fn try_from(value: SingleOrMultiple<String>) -> Result<Self, Self::Error> {
        Ok(match value {
            SingleOrMultiple::Single(value) => AllowOrigin::exact(value.parse()?),
            SingleOrMultiple::Multiple(value) => {
                AllowOrigin::list(value.into_iter().map(|v| v.parse()).collect::<Result<Vec<_>, _>>()?)
            }
        })
    }
}
