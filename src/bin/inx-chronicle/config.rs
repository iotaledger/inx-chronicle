// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::MongoDbConfig;

/// Configuration of Chronicle.
#[derive(Clone, Default, Debug)]
pub struct ChronicleConfig {
    pub mongodb: MongoDbConfig,
    #[cfg(feature = "influx")]
    pub influxdb: chronicle::db::influxdb::InfluxDbConfig,
    #[cfg(feature = "api")]
    pub api: crate::api::ApiConfig,
    #[cfg(feature = "inx")]
    pub inx: super::inx::InxConfig,
}
