// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::influxdb::{
    config::{DEFAULT_ANALYTICS_DATABASE_NAME, DEFAULT_ANALYTICS_ENABLED},
    AnalyticsChoice,
};
use clap::Args;

#[derive(Args, Debug)]
pub struct InfluxAnalyticsArgs {
    /// The Analytics database name.
    #[arg(long, value_name = "NAME", default_value = DEFAULT_ANALYTICS_DATABASE_NAME)]
    pub analytics_database_name: String,
    /// Disable InfluxDb time-series analytics writes.
    #[arg(long, default_value_t = !DEFAULT_ANALYTICS_ENABLED)]
    pub disable_analytics: bool,
    /// Select a subset of analytics to compute. If unset, all analytics will be computed.
    #[arg(long, value_name = "ANALYTICS")]
    pub analytics: Vec<AnalyticsChoice>,
}
