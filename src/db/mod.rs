// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the database and associated models.

/// Module containing InfluxDb types and traits.
#[cfg(feature = "influx")]
pub mod influxdb;
/// Module containing MongoDb types and traits.
pub mod mongodb;

pub use self::mongodb::{config::MongoDbConfig, MongoDb, MongoDbCollection, MongoDbCollectionExt};
