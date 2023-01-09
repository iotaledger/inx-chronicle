// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the database and associated models.

/// Module containing the collections in the database.
pub mod collections;

/// Module containing InfluxDb types and traits.
#[cfg(any(feature = "analytics", feature = "metrics"))]
pub mod influxdb;
/// Module containing MongoDb types and traits.
pub mod mongodb;

pub use self::mongodb::{config::MongoDbConfig, MongoDb, MongoDbCollection, MongoDbCollectionExt};
