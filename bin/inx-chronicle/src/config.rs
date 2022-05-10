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
#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChronicleConfig {
    pub mongodb: MongoDbConfig,
    #[cfg(feature = "api")]
    pub api: crate::api::ApiConfig,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub collector: crate::collector::CollectorConfig,
}

impl ChronicleConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        fs::read_to_string(&path)
            .map_err(ConfigError::FileRead)
            .and_then(|contents| toml::from_str::<Self>(&contents).map_err(ConfigError::TomlDeserialization))
    }

    /// Applies the appropriate command line arguments to the [`ChronicleConfig`].
    pub fn apply_cli_args(&mut self, args: super::CliArgs) {
        if let Some(connect_url) = args.db {
            self.mongodb = MongoDbConfig::new().with_connect_url(connect_url);
        }
        #[cfg(all(feature = "stardust", feature = "inx"))]
        match (args.solidifier_count, args.inx) {
            (Some(solidifier_count), Some(inx_config)) => {
                self.collector = crate::collector::CollectorConfig::new(
                    solidifier_count,
                    crate::collector::stardust_inx::StardustInxConfig::new(inx_config),
                );
            }
            (Some(solidifier_count), None) => {
                self.collector.solidifier_count = solidifier_count;
            }
            (None, Some(inx_config)) => {
                self.collector.inx = crate::collector::stardust_inx::StardustInxConfig::new(inx_config);
            }
            (None, None) => (),
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
