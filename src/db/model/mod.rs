// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::bson::Document;
use serde::{de::DeserializeOwned, Serialize};
/// Module containing the ledger inclusion state model.
pub mod inclusion_state;
#[cfg(feature = "stardust")]
/// Module containing Stardust data models.
pub mod stardust;
/// Module containing sync models.
pub mod sync;

/// Represents types that can be stored in the database.
pub trait Model: Serialize + DeserializeOwned {
    /// The name of the collection in the MongoDB database.
    const COLLECTION: &'static str;

    /// Gets the unique key of the model. Used for UPDATE/UPSERT operations.
    fn key(&self) -> Document;
}
