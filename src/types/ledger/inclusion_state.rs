use iota_types::api::dto as iota;
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
impl From<iota::LedgerInclusionStateDto> for LedgerInclusionState {
    fn from(value: iota::LedgerInclusionStateDto) -> Self {
        match value {
            iota::LedgerInclusionStateDto::Conflicting => Self::Conflicting,
            iota::LedgerInclusionStateDto::Included => Self::Included,
            iota::LedgerInclusionStateDto::NoTransaction => Self::NoTransaction,
        }
    }
}

#[cfg(feature = "stardust")]
impl From<LedgerInclusionState> for iota::LedgerInclusionStateDto {
    fn from(value: LedgerInclusionState) -> Self {
        match value {
            LedgerInclusionState::Conflicting => Self::Conflicting,
            LedgerInclusionState::Included => Self::Included,
            LedgerInclusionState::NoTransaction => Self::NoTransaction,
        }
    }
}
