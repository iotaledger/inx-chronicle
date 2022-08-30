// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing the collections in the database.
#[cfg(feature = "stardust")]
pub mod collections;

mod mongodb;

pub use self::mongodb::{MongoDb, MongoDbConfig};
