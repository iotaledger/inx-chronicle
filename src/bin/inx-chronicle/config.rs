// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::Path};

use chronicle::db::MongoDbConfig;
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
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChronicleConfig {
    pub mongodb: MongoDbConfig,
    #[cfg(feature = "api")]
    pub api: crate::api::ApiConfig,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub inx: super::stardust_inx::InxConfig,
    pub metrics: crate::metrics::MetricsConfig,
}

impl ChronicleConfig {
    /// Reads the config from the file located at `path`.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileRead(path.as_ref().display().to_string(), e))
            .and_then(|contents| toml::from_str::<Self>(&contents).map_err(ConfigError::TomlDeserialization))
    }

    /// Applies command line arguments to the config.
    pub fn apply_cl_args(&mut self, args: &super::cli::ClArgs) {
        if let Some(connect_url) = &args.db_addr {
            self.mongodb = MongoDbConfig {
                connect_url: connect_url.clone(),
                ..Default::default()
            };
        }
        #[cfg(all(feature = "stardust", feature = "inx"))]
        {
            if let Some(inx) = &args.inx_addr {
                self.inx.connect_url = inx.clone();
            }
            if let Some(enabled) = args.enable_inx {
                self.inx.enabled = enabled;
            }
        }
        #[cfg(feature = "api")]
        {
            if let Some(password) = &args.password {
                self.api.password_hash = hex::encode(
                    auth_helper::password::password_hash(password.as_bytes(), self.api.password_salt.as_bytes())
                        .expect("invalid JWT config"),
                );
            }
            if let Some(path) = &args.identity {
                self.api.identity_path.replace(path.clone());
            }
            if let Some(enabled) = args.enable_api {
                self.api.enabled = enabled;
            }
        }

        if let Some(enabled) = args.enable_metrics {
            self.metrics.enabled = enabled;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn config_file_conformity() -> Result<(), ConfigError> {
        let _ = ChronicleConfig::from_file(concat!(env!("CARGO_MANIFEST_DIR"), "config.template.toml"))?;

        Ok(())
    }
}
