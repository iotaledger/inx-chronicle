// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::bson::Document;
use serde::{de::DeserializeOwned, Serialize};
/// Module containing the ledger inclusion state model.
pub mod inclusion_state;
/// Module containing information about the network and state of the node.
pub mod status;

/// Module containing Stardust data models.
#[cfg(feature = "stardust")]
pub mod stardust;

/// Module containing sync models.
pub mod sync;

#[deprecated]
/// Represents types that can be stored in the database.
pub trait Model: Serialize + DeserializeOwned {
    /// The name of the collection in the MongoDB database.
    const COLLECTION: &'static str;

    /// Gets the unique key of the model. Used for UPDATE/UPSERT operations.
    fn key(&self) -> Document;
}

/// These are the names of the collections that we create in the database. At some point we could use a macro to make
/// this list nicer.
mod collection {
    pub const STATUS: &str = "status";
    pub const SYNC_RECORDS: &str = "sync_records";
}
