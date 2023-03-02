// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the time-series metrics model.

use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::{db::influxdb::InfluxDbMeasurement, model::tangle::MilestoneIndex};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, InfluxDbWriteable)]
#[allow(missing_docs)]
pub struct SyncMetrics {
    pub time: DateTime<Utc>,
    pub milestone_index: MilestoneIndex,
    pub milestone_time: u64,
    #[influxdb(tag)]
    pub chronicle_version: String,
}

#[cfg(feature = "analytics")]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, InfluxDbWriteable)]
#[allow(missing_docs)]
pub struct AnalyticsMetrics {
    pub time: DateTime<Utc>,
    pub milestone_index: MilestoneIndex,
    pub analytics_time: u64,
    #[influxdb(tag)]
    pub chronicle_version: String,
}

impl InfluxDbMeasurement for SyncMetrics {
    const NAME: &'static str = "sync_metrics";
}

#[cfg(feature = "analytics")]
impl InfluxDbMeasurement for AnalyticsMetrics {
    const NAME: &'static str = "analytics_metrics";
}
