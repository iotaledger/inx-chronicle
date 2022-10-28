// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the database and associated models.

/// Module containing the collections in the database.
#[cfg(feature = "stardust")]
pub mod collections;

#[cfg(feature = "influxdb")]
mod influxdb;
mod mongodb;

#[cfg(feature = "influxdb")]
pub use self::influxdb::{InfluxDb, InfluxDbConfig};
pub use self::mongodb::{MongoDb, MongoDbCollection, MongoDbCollectionExt, MongoDbConfig};
