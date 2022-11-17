// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use derive_more::From;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use tower_http::cors::AllowOrigin;

use super::{error::ConfigError, SecretKey};

/// API configuration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub argon_config: ArgonConfig,
}

impl ApiConfig {
    /// Applies the corresponding user config.
    #[allow(clippy::option_map_unit_fn)]
    pub fn apply_user_config(&mut self, user_config: ApiUserConfig) {
        user_config.enabled.map(|v| self.enabled = v);
        user_config.port.map(|v| self.port = v);
        user_config.allow_origins.map(|v| self.allow_origins = v);
        user_config.password_hash.map(|v| self.password_hash = v);
        user_config.password_salt.map(|v| self.password_salt = v);
        user_config.jwt_expiration.map(|v| self.jwt_expiration = v);
        user_config.public_routes.map(|v| self.public_routes = v);
        self.identity_path = user_config.identity_path;
        user_config.max_page_size.map(|v| self.max_page_size = v);
        user_config.argon_config.map(|v| self.argon_config.apply_user_config(v));
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiUserConfig {
    pub enabled: Option<bool>,
    pub port: Option<u16>,
    pub allow_origins: Option<SingleOrMultiple<String>>,
    pub password_hash: Option<String>,
    pub password_salt: Option<String>,
    #[serde(with = "humantime_serde")]
    pub jwt_expiration: Option<Duration>,
    pub public_routes: Option<Vec<String>>,
    pub identity_path: Option<String>,
    pub max_page_size: Option<usize>,
    pub argon_config: Option<ArgonUserConfig>,
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
    pub argon_config: ArgonConfig,
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
            argon_config: config.argon_config,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgonConfig {
    /// The length of the resulting hash.
    hash_length: u32,
    /// The number of lanes in parallel.
    parallelism: u32,
    /// The amount of memory requested (KB).
    mem_cost: u32,
    /// The number of passes.
    iterations: u32,
    /// The variant.
    #[serde(with = "variant")]
    variant: argon2::Variant,
    /// The version.
    #[serde(with = "version")]
    version: argon2::Version,
}

impl ArgonConfig {
    /// Applies the corresponding user config.
    #[allow(clippy::option_map_unit_fn)]
    pub fn apply_user_config(&mut self, user_config: ArgonUserConfig) {
        user_config.hash_length.map(|v| self.hash_length = v);
        user_config.parallelism.map(|v| self.parallelism = v);
        user_config
            .mem_cost
            .map(|v| self.mem_cost = v);
        user_config
            .iterations
            .map(|v| self.iterations = v);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArgonUserConfig {
    /// The length of the resulting hash.
    hash_length: Option<u32>,
    /// The number of lanes in parallel.
    parallelism: Option<u32>,
    /// The amount of memory requested (KB).
    mem_cost: Option<u32>,
    /// The number of passes.
    iterations: Option<u32>,
    /// The variant.
    #[serde(with = "variant")]
    variant: argon2::Variant,
    /// The version.
    #[serde(with = "version")]
    version: argon2::Version,
}

impl<'a> From<&'a ArgonConfig> for argon2::Config<'a> {
    fn from(val: &'a ArgonConfig) -> Self {
        Self {
            ad: &[],
            hash_length: val.hash_length,
            lanes: val.parallelism,
            mem_cost: val.mem_cost,
            secret: &[],
            thread_mode: Default::default(),
            time_cost: val.iterations,
            variant: val.variant,
            version: val.version,
        }
    }
}

mod variant {
    use serde::Deserialize;

    pub fn serialize<S: serde::Serializer>(val: &argon2::Variant, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&val.to_string())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<argon2::Variant, D::Error> {
        argon2::Variant::from_str(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

mod version {
    use serde::Deserialize;

    pub fn serialize<S: serde::Serializer>(val: &argon2::Version, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&format!("{:x}", val.as_u32()))
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<argon2::Version, D::Error> {
        let mut decoded = prefix_hex::decode::<Vec<u8>>(&String::deserialize(d)?).map_err(serde::de::Error::custom)?;
        decoded.resize(4, 0);
        argon2::Version::from_u32(u32::from_le_bytes(decoded.try_into().unwrap())).map_err(serde::de::Error::custom)
    }
}
