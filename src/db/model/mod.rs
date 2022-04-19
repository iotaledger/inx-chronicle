// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "stardust")]
/// Models for Stardust layouts.
pub mod stardust;

use serde::{Serialize, de::DeserializeOwned};

/// Represents types that can be stored in the database.
pub trait Model: Serialize + DeserializeOwned {
    /// The name of the collection in the MongoDB database.
    const COLLECTION: &'static str;

    /// The type behind the MongoDB `_id` field.
    type Id: Serialize;
}
