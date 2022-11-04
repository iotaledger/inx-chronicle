// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::db::influxdb::InfluxDbMeasurement;

impl InfluxDbMeasurement for SyncMetrics {
    const NAME: &'static str = "sync_metrics";
}
