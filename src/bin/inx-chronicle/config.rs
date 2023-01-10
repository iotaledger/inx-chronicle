// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::MongoDbConfig;
use serde::{Deserialize, Serialize};

/// Configuration of Chronicle.
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChronicleConfig {
    pub mongodb: MongoDbConfig,
    #[cfg(any(feature = "analytics", feature = "metrics"))]
    pub influxdb: chronicle::db::influxdb::InfluxDbConfig,
    #[cfg(feature = "api")]
    pub api: crate::api::ApiConfig,
    #[cfg(feature = "inx")]
    pub inx: super::stardust_inx::InxConfig,
    #[cfg(feature = "loki")]
    pub loki: loki::LokiConfig,
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
