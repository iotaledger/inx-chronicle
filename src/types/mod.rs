// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod message;
mod mongo;
pub mod sync;

use anyhow::*;
use serde::{
    Deserialize,
    Serialize,
};

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
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Conflicting),
            1 => Ok(Self::Included),
            2 => Ok(Self::NoTransaction),
            n => bail!("Unexpected ledger inclusion byte state: {}", n),
        }
    }
}

impl From<crate::cpt2::types::dtos::LedgerInclusionStateDto> for LedgerInclusionState {
    fn from(value: crate::cpt2::types::dtos::LedgerInclusionStateDto) -> Self {
        match value {
            crate::cpt2::types::dtos::LedgerInclusionStateDto::Conflicting => Self::Conflicting,
            crate::cpt2::types::dtos::LedgerInclusionStateDto::Included => Self::Included,
            crate::cpt2::types::dtos::LedgerInclusionStateDto::NoTransaction => Self::NoTransaction,
        }
    }
}

impl Into<crate::cpt2::types::dtos::LedgerInclusionStateDto> for LedgerInclusionState {
    fn into(self) -> crate::cpt2::types::dtos::LedgerInclusionStateDto {
        match self {
            Self::Conflicting => crate::cpt2::types::dtos::LedgerInclusionStateDto::Conflicting,
            Self::Included => crate::cpt2::types::dtos::LedgerInclusionStateDto::Included,
            Self::NoTransaction => crate::cpt2::types::dtos::LedgerInclusionStateDto::NoTransaction,
        }
    }
}

impl From<crate::shimmer::types::dtos::LedgerInclusionStateDto> for LedgerInclusionState {
    fn from(value: crate::shimmer::types::dtos::LedgerInclusionStateDto) -> Self {
        match value {
            crate::shimmer::types::dtos::LedgerInclusionStateDto::Conflicting => Self::Conflicting,
            crate::shimmer::types::dtos::LedgerInclusionStateDto::Included => Self::Included,
            crate::shimmer::types::dtos::LedgerInclusionStateDto::NoTransaction => Self::NoTransaction,
        }
    }
}

impl Into<crate::shimmer::types::dtos::LedgerInclusionStateDto> for LedgerInclusionState {
    fn into(self) -> crate::shimmer::types::dtos::LedgerInclusionStateDto {
        match self {
            Self::Conflicting => crate::shimmer::types::dtos::LedgerInclusionStateDto::Conflicting,
            Self::Included => crate::shimmer::types::dtos::LedgerInclusionStateDto::Included,
            Self::NoTransaction => crate::shimmer::types::dtos::LedgerInclusionStateDto::NoTransaction,
        }
    }
}
