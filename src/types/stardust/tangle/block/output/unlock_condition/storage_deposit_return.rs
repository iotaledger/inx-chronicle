// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use super::OutputAmount;
use crate::types::{context::TryFromWithContext, stardust::tangle::block::Address};

/// Defines the amount of tokens used as storage deposit that have to be returned to the return address.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDepositReturnUnlockCondition {
    /// The address to which funds will be returned once the storage deposit is unlocked.
    pub return_address: Address,
    /// The amount held in storage.
    pub amount: OutputAmount,
}

impl<T: Borrow<iota::StorageDepositReturnUnlockCondition>> From<T> for StorageDepositReturnUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            return_address: value.borrow().return_address().into(),
            amount: value.borrow().amount().into(),
        }
    }
}

impl TryFromWithContext<StorageDepositReturnUnlockCondition> for iota::StorageDepositReturnUnlockCondition {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: StorageDepositReturnUnlockCondition,
    ) -> Result<Self, Self::Error> {
        iota::StorageDepositReturnUnlockCondition::new(value.return_address.into(), value.amount.0, ctx.token_supply())
    }
}

impl From<StorageDepositReturnUnlockCondition> for iota::dto::StorageDepositReturnUnlockConditionDto {
    fn from(value: StorageDepositReturnUnlockCondition) -> Self {
        Self {
            kind: iota::StorageDepositReturnUnlockCondition::KIND,
            return_address: value.return_address.into(),
            amount: value.amount.0.to_string(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use super::*;

    impl StorageDepositReturnUnlockCondition {
        /// Generates a random [`StorageDepositReturnUnlockCondition`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self {
                return_address: Address::rand_ed25519(),
                amount: OutputAmount::rand(ctx),
            }
        }
    }
}
