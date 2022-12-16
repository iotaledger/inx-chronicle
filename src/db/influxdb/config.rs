// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `InfluxDb` config and its defaults.

use serde::{Deserialize, Serialize};

/// The default InfluxDb URL to connect to.
pub const DEFAULT_CONN_URL: &str = "http://localhost:8086";
/// The default InfluxDb username.
pub const DEFAULT_USERNAME: &str = "root";
/// The default InfluxDb password.
pub const DEFAULT_PASSWORD: &str = "password";
/// The default whether to enable influx analytics writes.
pub const DEFAULT_ANALYTICS_ENABLED: bool = true;
/// The default name of the analytics database to connect to.
pub const DEFAULT_ANALYTICS_DATABASE_NAME: &str = "chronicle_analytics";
/// The default whether to enable influx metrics writes.
pub const DEFAULT_METRICS_ENABLED: bool = true;
/// The default name of the metrics database to connect to.
pub const DEFAULT_METRICS_DATABASE_NAME: &str = "chronicle_metrics";

/// The influxdb [`Client`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InfluxDbConfig {
    /// The address of the InfluxDb instance.
    pub conn_url: String,
    /// The InfluxDb username.
    pub username: String,
    /// The InfluxDb password.
    pub password: String,
    /// Whether to enable influx analytics writes.
    pub analytics_enabled: bool,
    /// The name of the database to insert analytics.
    pub analytics_database_name: String,
    /// Whether to enable influx metrics writes.
    pub metrics_enabled: bool,
    /// The name of the database to insert metrics.
    pub metrics_database_name: String,
}

impl Default for InfluxDbConfig {
    fn default() -> Self {
        Self {
            conn_url: DEFAULT_CONN_URL.to_string(),
            username: DEFAULT_USERNAME.to_string(),
            password: DEFAULT_PASSWORD.to_string(),
            analytics_enabled: DEFAULT_ANALYTICS_ENABLED,
            analytics_database_name: DEFAULT_ANALYTICS_DATABASE_NAME.to_string(),
            metrics_enabled: DEFAULT_METRICS_ENABLED,
            metrics_database_name: DEFAULT_METRICS_DATABASE_NAME.to_string(),
        }
    }
}