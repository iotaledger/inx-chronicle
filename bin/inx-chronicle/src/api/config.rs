// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::From;
use libp2p_core::identity::ed25519::SecretKey;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use tower_http::cors::AllowOrigin;

use super::error::ConfigError;

/// API configuration
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub port: u16,
    pub allow_origins: Option<SingleOrMultiple<String>>,
    pub password_hash: String,
    pub jwt_salt: String,
    pub public_routes: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: 9092,
            allow_origins: Some(String::from("*").into()),
            password_hash: "0000".to_string(),
            jwt_salt: String::from("Chronicle"),
            public_routes: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApiData {
    pub port: u16,
    pub allow_origins: Option<SingleOrMultiple<String>>,
    pub password_hash: String,
    pub jwt_salt: String,
    pub public_routes: RegexSet,
    pub secret_key: SecretKey,
}

impl ApiData {
    pub const ISSUER: &'static str = "chronicle";
    pub const AUDIENCE: &'static str = "api";
}

impl TryFrom<(ApiConfig, SecretKey)> for ApiData {
    type Error = ConfigError;

    fn try_from((config, secret_key): (ApiConfig, SecretKey)) -> Result<Self, Self::Error> {
        Ok(Self {
            port: config.port,
            allow_origins: config.allow_origins,
            password_hash: config.password_hash,
            jwt_salt: config.jwt_salt,
            public_routes: RegexSet::new(config.public_routes)?,
            secret_key,
        })
    }
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
