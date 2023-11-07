// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the types.

pub mod address;
pub mod raw;
pub mod tag;

use mongodb::bson::Bson;
use serde::Serialize;

/// Helper trait for serializable types
pub trait SerializeToBson: Serialize {
    /// Serializes values to Bson infallibly
    fn to_bson(&self) -> Bson {
        mongodb::bson::to_bson(self).unwrap()
    }
}
impl<T: Serialize> SerializeToBson for T {}
