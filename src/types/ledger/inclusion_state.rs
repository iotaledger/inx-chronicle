// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::api::dto as bee;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

/// A block's ledger inclusion state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerInclusionState {
    /// A conflicting block, ex. a double spend
    Conflicting,
    /// A successful, included block
    Included,
    /// A block without a transaction
    NoTransaction,
}

impl From<LedgerInclusionState> for Bson {
    fn from(val: LedgerInclusionState) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&val).unwrap()
    }
}

#[cfg(feature = "stardust")]
impl From<bee::LedgerInclusionStateDto> for LedgerInclusionState {
    fn from(value: bee::LedgerInclusionStateDto) -> Self {
        match value {
            bee::LedgerInclusionStateDto::Conflicting => Self::Conflicting,
            bee::LedgerInclusionStateDto::Included => Self::Included,
            bee::LedgerInclusionStateDto::NoTransaction => Self::NoTransaction,
        }
    }
}

#[cfg(feature = "stardust")]
impl From<LedgerInclusionState> for bee::LedgerInclusionStateDto {
    fn from(value: LedgerInclusionState) -> Self {
        match value {
            LedgerInclusionState::Conflicting => Self::Conflicting,
            LedgerInclusionState::Included => Self::Included,
            LedgerInclusionState::NoTransaction => Self::NoTransaction,
        }
    }
}
