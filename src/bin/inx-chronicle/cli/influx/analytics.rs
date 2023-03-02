// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::influxdb::AnalyticsChoice;

use super::*;

#[derive(Args, Debug)]
pub struct InfluxAnalyticsArgs {
    /// The Analytics database name.
    #[arg(long, value_name = "NAME", default_value = influxdb::DEFAULT_ANALYTICS_DATABASE_NAME)]
    pub analytics_database_name: String,
    /// Disable InfluxDb time-series analytics writes.
    #[arg(long, default_value_t = !influxdb::DEFAULT_ANALYTICS_ENABLED)]
    pub disable_analytics: bool,
    /// Select a subset of analytics to compute. If unset, all analytics will be computed.
    #[arg(long, value_name = "ANALYTICS")]
    pub analytics: Vec<AnalyticsChoice>,
}
