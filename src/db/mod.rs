// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing database record models.
pub mod model;
pub mod mongodb;

pub use self::mongodb::{MongoDb, MongoDbConfig};
