// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the types.

// pub mod block;
// pub mod node;
// pub mod protocol;
// pub mod signature;
// pub mod util;

// pub use block::*;
// pub use node::*;
// pub use protocol::*;
// pub use signature::*;
// pub use util::*;

// pub mod utxo {
//     //! A logical grouping of UTXO types for convenience.
//     #![allow(ambiguous_glob_reexports)]
//     pub use super::block::payload::transaction::{
//         input::*,
//         output::{address::*, unlock_condition::*, *},
//         unlock::*,
//     };
// }
// // Bring this module up to the top level for convenience
// pub use self::block::payload::transaction::output::ledger;
// pub mod metadata {
//     //! A logical grouping of metadata types for convenience.
//     pub use super::{block::metadata::*, utxo::metadata::*};
// }
// pub mod tangle {
//     //! A logical grouping of ledger types for convenience.
//     pub use super::block::payload::milestone::{MilestoneIndex, MilestoneIndexTimestamp, MilestoneTimestamp};
// }

use mongodb::bson::Bson;
use serde::{de::DeserializeOwned, Serialize};

/// Helper trait for serializable types
pub trait SerializeToBson: Serialize {
    /// Serializes values to Bson infallibly
    fn to_bson(&self) -> Bson {
        mongodb::bson::to_bson(self).unwrap()
    }
}
impl<T: Serialize> SerializeToBson for T {}

/// Helper trait for deserializable types
pub trait DeserializeFromBson: DeserializeOwned {
    /// Serializes values to Bson infallibly
    fn from_bson(bson: Bson) -> mongodb::bson::de::Result<Self>
    where
        Self: Sized,
    {
        mongodb::bson::from_bson(bson)
    }
}
impl<T: DeserializeOwned> DeserializeFromBson for T {}
