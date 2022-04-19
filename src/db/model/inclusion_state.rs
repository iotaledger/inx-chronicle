// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Unexpected ledger inclusion state: {0}")]
#[allow(missing_docs)]
pub struct UnexpectedLedgerInclusionState(u8);

/// A message's ledger inclusion state
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum LedgerInclusionState {
    /// A conflicting message, ex. a double spend
    #[serde(rename = "conflicting")]
    Conflicting = 0,
    /// A successful, included message
    #[serde(rename = "included")]
    Included = 1,
    /// A message without a transaction
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

#[cfg(feature = "stardust")]
impl From<crate::stardust::types::dtos::LedgerInclusionStateDto> for LedgerInclusionState {
    fn from(value: crate::stardust::types::dtos::LedgerInclusionStateDto) -> Self {
        match value {
            crate::stardust::types::dtos::LedgerInclusionStateDto::Conflicting => Self::Conflicting,
            crate::stardust::types::dtos::LedgerInclusionStateDto::Included => Self::Included,
            crate::stardust::types::dtos::LedgerInclusionStateDto::NoTransaction => Self::NoTransaction,
        }
    }
}

#[cfg(feature = "stardust")]
impl From<LedgerInclusionState> for crate::stardust::types::dtos::LedgerInclusionStateDto {
    fn from(v: LedgerInclusionState) -> Self {
        match v {
            LedgerInclusionState::Conflicting => Self::Conflicting,
            LedgerInclusionState::Included => Self::Included,
            LedgerInclusionState::NoTransaction => Self::NoTransaction,
        }
    }
}
