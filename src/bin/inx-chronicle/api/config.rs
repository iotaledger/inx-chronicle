// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use derive_more::From;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use tower_http::cors::AllowOrigin;

use super::{error::ConfigError, SecretKey};

pub const DEFAULT_ENABLED: bool = true;
pub const DEFAULT_PORT: u16 = 8042;
pub const DEFAULT_ALLOW_ORIGINS: &str = "0.0.0.0";
pub const DEFAULT_PUBLIC_ROUTES: &str = "api/core/v2/*";
pub const DEFAULT_MAX_PAGE_SIZE: usize = 1000;
pub const DEFAULT_JWT_PASSWORD: &str = "password";
pub const DEFAULT_JWT_SALT: &str = "saltines";
pub const DEFAULT_JWT_EXPIRATIOIN: &str = "72h";

/// API configuration
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiConfig {
    pub enabled: bool,
    pub port: u16,
    pub allow_origins: SingleOrMultiple<String>,
    pub public_routes: Vec<String>,
    pub max_page_size: usize,
    pub identity_path: Option<String>,
    pub password_hash: String,
    pub password_salt: String,
    #[serde(with = "humantime_serde")]
    pub jwt_expiration: Duration,
    pub argon_config: ArgonConfig,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_ENABLED,
            port: DEFAULT_PORT,
            allow_origins: SingleOrMultiple::Single(DEFAULT_ALLOW_ORIGINS.to_string()),
            public_routes: vec![DEFAULT_PUBLIC_ROUTES.to_string()],
            max_page_size: DEFAULT_MAX_PAGE_SIZE,
            identity_path: None,
            password_hash: DEFAULT_JWT_PASSWORD.to_string(),
            password_salt: DEFAULT_JWT_SALT.to_string(),
            jwt_expiration: DEFAULT_JWT_EXPIRATIOIN
                .parse::<humantime::Duration>()
                .map(Into::into)
                .unwrap(),
            argon_config: ArgonConfig::default(),
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

impl<T: Default> Default for SingleOrMultiple<T> {
    fn default() -> Self {
        Self::Single(Default::default())
    }
}

impl<T: Clone> From<&Vec<T>> for SingleOrMultiple<T> {
    fn from(value: &Vec<T>) -> Self {
        if value.is_empty() {
            unreachable!("Vec must have single or multiple elements")
        } else if value.len() == 1 {
            Self::Single(value[0].clone())
        } else {
            Self::Multiple(value.to_vec())
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
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

impl Default for ArgonConfig {
    fn default() -> Self {
        Self {
            hash_length: 32,
            parallelism: 1,
            mem_cost: 4096,
            iterations: 3,
            variant: Default::default(),
            version: Default::default(),
        }
    }
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
