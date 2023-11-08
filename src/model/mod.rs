// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains model types.

pub mod address;
pub mod block_metadata;
pub mod ledger;
pub mod native_token;
pub mod node;
pub mod protocol;
pub mod raw;
pub mod slot;
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
