// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod error;

/// Module that contains utilities to serialize values to BSON.
pub mod bson;
/// Module containing database record models.
pub mod model;
pub mod mongodb;

pub use self::{
    error::MongoDbError,
    mongodb::{MongoDb, MongoDbConfig},
};
