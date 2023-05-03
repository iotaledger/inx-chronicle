// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "analytics")]
mod analytics;
#[cfg(feature = "metrics")]
mod metrics;

use chronicle::db::influxdb::{config as influxdb, InfluxDbConfig};
use clap::Args;

#[derive(Args, Debug)]
pub struct InfluxDbArgs {
    /// The url pointing to an InfluxDb instance.
    #[arg(long, value_name = "URL", env = "INFLUXDB_URL", default_value = influxdb::DEFAULT_URL)]
    pub influxdb_url: String,
    /// The InfluxDb username.
    #[arg(long, value_name = "USERNAME", env = "INFLUXDB_USERNAME", default_value = influxdb::DEFAULT_USERNAME)]
    pub influxdb_username: String,
    /// The maximum number of attempts pushing measurements to InfluxDb.
    #[arg(long, value_name = "NUM", default_value_t = 3)]
    pub influxdb_max_retries: usize,
    /// The InfluxDb password.
    #[arg(long, value_name = "PASSWORD", env = "INFLUXDB_PASSWORD", default_value = influxdb::DEFAULT_PASSWORD)]
    pub influxdb_password: String,
    #[cfg(feature = "analytics")]
    #[command(flatten)]
    pub analytics_args: analytics::InfluxAnalyticsArgs,
    #[cfg(feature = "metrics")]
    #[command(flatten)]
    pub metrics_args: metrics::InfluxMetricsArgs,
}

impl From<&InfluxDbArgs> for InfluxDbConfig {
    fn from(value: &InfluxDbArgs) -> Self {
        Self {
            url: value.influxdb_url.clone(),
            username: value.influxdb_username.clone(),
            password: value.influxdb_password.clone(),
            max_retries: value.influxdb_max_retries,
            #[cfg(feature = "analytics")]
            analytics_enabled: !value.analytics_args.disable_analytics,
            #[cfg(feature = "analytics")]
            analytics_database_name: value.analytics_args.analytics_database_name.clone(),
            #[cfg(feature = "analytics")]
            analytics: value.analytics_args.analytics.clone(),
            #[cfg(feature = "metrics")]
            metrics_enabled: !value.metrics_args.disable_metrics,
            #[cfg(feature = "metrics")]
            metrics_database_name: value.metrics_args.metrics_database_name.clone(),
        }
    }
}
