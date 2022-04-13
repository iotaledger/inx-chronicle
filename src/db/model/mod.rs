// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "stardust")]
pub mod stardust;

use serde::Serialize;

// TODO: Add `serde::Deserialize` constraint.
/// Represents types that can be stored in the database.
pub trait Model: Serialize {
    /// The name of the collection in the MongoDB database.
    const COLLECTION: &'static str;
}
