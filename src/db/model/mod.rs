// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

// TODO: Add Chrysalis types analogously.

#[cfg(feature = "stardust")]
/// Model for Stardust types.
pub mod stardust;

// TODO: Add `serde::Deserialize` constraint.
/// Represents types that can be stored in the database.
pub trait Model: Serialize {
    /// The name of the collection in the MongoDB database.
    const COLLECTION: &'static str;
}
