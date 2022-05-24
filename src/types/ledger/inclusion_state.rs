// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_rest_api_stardust::types::dtos as bee;
use serde::{Deserialize, Serialize};

/// A block's ledger inclusion state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LedgerInclusionState {
    /// A conflicting block, ex. a double spend
    #[serde(rename = "conflicting")]
    Conflicting,
    /// A successful, included block
    #[serde(rename = "included")]
    Included,
    /// A block without a transaction
    #[serde(rename = "noTransaction")]
    NoTransaction,
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

#[cfg(feature = "inx")]
impl From<inx::LedgerInclusionState> for LedgerInclusionState {
    fn from(value: inx::LedgerInclusionState) -> Self {
        match value {
            inx::LedgerInclusionState::Included => Self::Included,
            inx::LedgerInclusionState::NoTransaction => Self::NoTransaction,
            inx::LedgerInclusionState::Conflicting => Self::Conflicting,
        }
    }
}
