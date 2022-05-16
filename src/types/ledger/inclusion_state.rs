// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_rest_api_stardust::types::dtos as bee;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Unexpected ledger inclusion state: {0}")]
#[allow(missing_docs)]
pub struct UnexpectedLedgerInclusionState(u8);

/// A block's ledger inclusion state.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum LedgerInclusionState {
    /// A conflicting block, ex. a double spend
    #[serde(rename = "conflicting")]
    Conflicting = 0,
    /// A successful, included block
    #[serde(rename = "included")]
    Included = 1,
    /// A block without a transaction
    #[serde(rename = "noTransaction")]
    NoTransaction = 2,
}

impl TryFrom<u8> for LedgerInclusionState {
    type Error = UnexpectedLedgerInclusionState;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Conflicting),
            1 => Ok(Self::Included),
            2 => Ok(Self::NoTransaction),
            n => Err(UnexpectedLedgerInclusionState(n)),
        }
    }
}

impl From<LedgerInclusionState> for Bson {
    fn from(l: LedgerInclusionState) -> Self {
        Bson::Int32((l as u8).into())
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
