//! Module that contains the database and associated models.

/// Module containing the collections in the database.
#[cfg(feature = "stardust")]
pub mod collections;

/// Module containing InfluxDb types and traits.
#[cfg(any(feature = "analytics", feature = "metrics"))]
pub mod influxdb;
/// Module containing MongoDb types and traits.
pub mod mongodb;

pub use self::mongodb::{config::MongoDbConfig, MongoDb, MongoDbCollection, MongoDbCollectionExt};
