// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the types.

pub mod address;
pub mod block;
pub mod node;
pub mod protocol;
pub mod signature;
pub mod util;

pub use address::*;
pub use block::*;
pub use node::*;
pub use protocol::*;
pub use signature::*;
pub use util::*;

pub mod utxo {
    //! A logical grouping of UTXO types for convenience.
    pub use super::block::payload::transaction::{
        input::*,
        output::{unlock_condition::*, *},
        unlock::*,
    };
}
// Bring this module up to the top level for convenience
pub use self::block::payload::transaction::output::ledger;
pub mod metadata {
    //! A logical grouping of metadata types for convenience.
    pub use super::{block::metadata::*, utxo::metadata::*};
}
pub mod tangle {
    //! A logical grouping of ledger types for convenience.
    pub use super::block::payload::milestone::{MilestoneIndex, MilestoneIndexTimestamp, MilestoneTimestamp};
}
