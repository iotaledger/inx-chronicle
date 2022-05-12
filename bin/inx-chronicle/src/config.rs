// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::Path};

use chronicle::db::MongoDbConfig;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read file: {0}")]
    FileRead(std::io::Error),
    #[error("toml deserialization failed: {0}")]
    TomlDeserialization(toml::de::Error),
}

/// Configuration of Chronicle.
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChronicleConfig {
    #[serde(default)]
    pub mongodb: MongoDbConfig,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    #[serde(default)]
    pub inx: crate::stardust_inx::InxConfig,
    #[cfg(feature = "api")]
    #[serde(default)]
    pub api: crate::api::ApiConfig,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    #[serde(default)]
    pub collector: crate::collector::CollectorConfig,
    #[cfg(feature = "metrics")]
    #[serde(default)]
    pub metrics: crate::metrics::MetricsConfig,
}

impl ChronicleConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        fs::read_to_string(&path)
            .map_err(ConfigError::FileRead)
            .and_then(|contents| toml::from_str::<Self>(&contents).map_err(ConfigError::TomlDeserialization))
    }

    /// Applies the appropriate command line arguments to the [`ChronicleConfig`].
    pub fn apply_cli_args(&mut self, args: super::cli::CliArgs) {
        #[cfg(all(feature = "stardust", feature = "inx"))]
        if let Some(inx) = args.inx {
            self.inx = crate::stardust_inx::InxConfig::new(inx);
        }
        if let Some(connect_url) = args.db {
            self.mongodb = MongoDbConfig::new().with_connect_url(connect_url);
        }
        #[cfg(all(feature = "stardust", feature = "inx"))]
        if let Some(solidifier_count) = args.solidifier_count {
            self.collector = crate::collector::CollectorConfig::new(solidifier_count);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn config_file_conformity() -> Result<(), ConfigError> {
        let _ = ChronicleConfig::from_file(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/bin/inx-chronicle/config.template.toml"
        ))?;

        Ok(())
    }
}
