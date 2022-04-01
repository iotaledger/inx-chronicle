// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::Path};

use chronicle::{db::MongoConfig, inx::InxConfig};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("toml deserialization failed: {0}")]
    TomlDeserialization(toml::de::Error),
    #[error("failed to read file: {0}")]
    FileRead(std::io::Error),
}

/// Configuration of Chronicle.
#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub mongodb: MongoConfig,
    pub inx: InxConfig,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        fs::read_to_string(&path)
            .map_err(ConfigError::FileRead)
            .and_then(|contents| toml::from_str::<Self>(&contents).map_err(ConfigError::TomlDeserialization))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn config_file_conformity() -> Result<(), ConfigError> {
        let _ = Config::from_file(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml"))?;

        Ok(())
    }
}
