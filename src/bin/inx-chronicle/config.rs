// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::Path};

use chronicle::db::{MongoDbConfig, MongoDbUserConfig};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[cfg(feature = "api")]
    #[error(transparent)]
    Api(#[from] crate::api::ConfigError),
    #[error("failed to read config at '{0}': {1}")]
    FileRead(String, std::io::Error),
    #[error("toml deserialization failed: {0}")]
    TomlDeserialization(toml::de::Error),
}

/// Configuration of Chronicle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChronicleConfig {
    pub mongodb: MongoDbConfig,
    #[cfg(any(feature = "analytics", feature = "metrics"))]
    pub influxdb: chronicle::db::influxdb::InfluxDbConfig,
    #[cfg(feature = "api")]
    pub api: crate::api::ApiConfig,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub inx: super::stardust_inx::InxConfig,
    #[cfg(feature = "loki")]
    pub loki: LokiConfig,
}

impl ChronicleConfig {
    /// Reads the config from the file located at `path`.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileRead(path.as_ref().display().to_string(), e))
            .and_then(|contents| toml::from_str::<Self>(&contents).map_err(ConfigError::TomlDeserialization))
    }

    /// Applies the corresponding user config.
    #[allow(clippy::option_map_unit_fn)]
    pub fn apply_user_config(&mut self, user_config: ChronicleUserConfig) {
        user_config
            .influxdb
            .map(|c| self.influxdb.apply_user_config(c));
        user_config.api.map(|c| self.api.apply_user_config(c));
        user_config.inx.map(|c| self.inx.apply_user_config(c));
        user_config.loki.map(|c| self.loki.apply_user_config(c));
    }
}

/// Configuration of Chronicle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChronicleUserConfig {
    pub mongodb: Option<MongoDbUserConfig>,
    #[cfg(any(feature = "analytics", feature = "metrics"))]
    pub influxdb: Option<chronicle::db::influxdb::InfluxDbUserConfig>,
    #[cfg(feature = "api")]
    pub api: Option<crate::api::ApiUserConfig>,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub inx: Option<super::stardust_inx::InxUserConfig>,
    #[cfg(feature = "loki")]
    pub loki: Option<LokiUserConfig>,
}

impl ChronicleUserConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileRead(path.as_ref().display().to_string(), e))
            .and_then(|contents| toml::from_str::<Self>(&contents).map_err(ConfigError::TomlDeserialization))
    }
}

#[cfg(feature = "loki")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LokiConfig {
    pub enabled: bool,
    pub connect_url: String,
}

impl LokiConfig {
    /// Applies the corresponding user config.
    #[allow(clippy::option_map_unit_fn)]
    pub fn apply_user_config(&mut self, user_config: LokiUserConfig) {
        user_config.enabled.map(|v| self.enabled = v);
        user_config.connect_url.map(|v| self.connect_url = v);
    }
}

#[cfg(feature = "loki")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LokiUserConfig {
    pub enabled: Option<bool>,
    pub connect_url: Option<String>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn config_file_conformity() -> Result<(), ConfigError> {
        let _ = ChronicleConfig::from_file(concat!(env!("CARGO_MANIFEST_DIR"), "/config.defaults.toml"))?;

        Ok(())
    }
}
