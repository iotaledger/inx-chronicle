// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use influxdb::InfluxDbWriteable;

/// An InfluxDb measurement.
pub trait InfluxDbMeasurement: InfluxDbWriteable + Send + Sync {
    /// The measurement name.
    const NAME: &'static str;
}
