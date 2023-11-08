// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::influxdb::config::{DEFAULT_METRICS_DATABASE_NAME, DEFAULT_METRICS_ENABLED};
use clap::Args;

#[derive(Args, Debug)]
pub struct InfluxMetricsArgs {
    /// The Metrics database name.
    #[arg(long, value_name = "NAME", default_value = DEFAULT_METRICS_DATABASE_NAME)]
    pub metrics_database_name: String,
    /// Disable InfluxDb time-series metrics writes.
    #[arg(long, default_value_t = !DEFAULT_METRICS_ENABLED)]
    pub disable_metrics: bool,
}
