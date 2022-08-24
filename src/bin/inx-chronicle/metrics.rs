// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use metrics_exporter_prometheus::{BuildError, PrometheusBuilder};
use metrics_util::MetricKindMask;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub address: IpAddr,
    pub port: u16,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            address: [0, 0, 0, 0].into(),
            port: 9100,
        }
    }
}

pub fn setup(config: &MetricsConfig) -> Result<(), BuildError> {
    let addr = SocketAddr::new(config.address, config.port);

    let builder = PrometheusBuilder::new();
    builder
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::HISTOGRAM,
            Some(Duration::from_secs(10)),
        )
        .with_http_listener(addr)
        .install()
}
