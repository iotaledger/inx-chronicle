// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::Path};

use chronicle::db::MongoDbConfig;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config at '{0}': {1}")]
    FileRead(String, std::io::Error),
    #[error("toml deserialization failed: {0}")]
    TomlDeserialization(toml::de::Error),
}

/// Configuration of Chronicle.
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChronicleConfig {
    pub mongodb: MongoDbConfig,
    #[cfg(any(feature = "analytics", feature = "metrics"))]
    pub influxdb: chronicle::db::influxdb::InfluxDbConfig,
    #[cfg(feature = "api")]
    pub api: crate::api::ApiConfig,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub inx: super::stardust_inx::InxConfig,
    #[cfg(feature = "loki")]
    pub loki: loki::LokiConfig,
}

impl ChronicleConfig {
    /// Reads the config from the file located at `path`.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileRead(path.as_ref().display().to_string(), e))
            .and_then(|contents| toml::from_str::<Self>(&contents).map_err(ConfigError::TomlDeserialization))
    }
}

#[cfg(feature = "loki")]
pub mod loki {
    use super::*;

    pub const DEFAULT_LOKI_ENABLED: bool = true;
    pub const DEFAULT_LOKI_URL: &str = "http://localhost:3100";

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct LokiConfig {
        pub enabled: bool,
        pub url: String,
    }

    impl Default for LokiConfig {
        fn default() -> Self {
            Self {
                enabled: DEFAULT_LOKI_ENABLED,
                url: DEFAULT_LOKI_URL.to_string(),
            }
        }
    }
}
