// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::Measurement;
use crate::db::influxdb::InfluxDb;

// TODO abstraction that runs all selected analytics concurrently

impl InfluxDb {
    /// TODO: Rename
    pub async fn insert_measurement(&self, measurement: Box<dyn Measurement>) -> Result<(), influxdb::Error> {
        self.analytics().query(measurement.into_write_query()).await?;
        Ok(())
    }
}
