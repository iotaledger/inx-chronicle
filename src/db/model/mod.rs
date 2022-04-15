// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use mongodb::bson::Document;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[cfg(feature = "chrysalis")]
/// Module containing Chrysalis data models.
pub mod chrysalis;
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

/// Represents errors that happened during conversion from INX proto types to model types.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum InxConversionError {
    #[error("Missing field {0}")]
    MissingField(&'static str),
    #[error("Invalid field {0}")]
    InvalidField(&'static str),
    #[error("Invalid buffer length")]
    InvalidBufferLength,
    #[error(transparent)]
    PackableError(Box<dyn Error + Send + Sync>),
}
