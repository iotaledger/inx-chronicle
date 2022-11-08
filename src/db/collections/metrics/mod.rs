// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Schema implementation for InfluxDb.
pub mod influx;

use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::types::tangle::MilestoneIndex;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, InfluxDbWriteable)]
#[allow(missing_docs)]
pub struct SyncMetrics {
    pub time: DateTime<Utc>,
    pub milestone_index: MilestoneIndex,
    pub sync_time: u64,
    #[influxdb(tag)]
    pub chronicle_version: String,
    #[influxdb(tag)]
    pub network_name: String,
}
