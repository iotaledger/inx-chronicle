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
}
