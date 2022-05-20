// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as stardust;
use serde::{Deserialize, Serialize};

use super::AliasId;
use crate::types::stardust::block::Address;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UnlockCondition {
    #[serde(rename = "address")]
    Address(Address),
    #[serde(rename = "storage_deposit_return")]
    StorageDepositReturn {
        return_address: Address,
        #[serde(with = "crate::types::stringify")]
        amount: u64,
    },
    #[serde(rename = "timelock")]
    Timelock { milestone_index: u32, timestamp: u32 },
    #[serde(rename = "expiration")]
    Expiration {
        return_address: Address,
        milestone_index: u32,
        timestamp: u32,
    },
    #[serde(rename = "state_controller_address")]
    StateControllerAddress(Address),
    #[serde(rename = "governor_address")]
    GovernorAddress(Address),
    #[serde(rename = "immutable_alias_address")]
    ImmutableAliasAddress(AliasId),
}

impl From<&stardust::UnlockCondition> for UnlockCondition {
    fn from(value: &stardust::UnlockCondition) -> Self {
        match value {
            stardust::UnlockCondition::Address(a) => Self::Address(a.address().into()),
            stardust::UnlockCondition::StorageDepositReturn(c) => Self::StorageDepositReturn {
                return_address: c.return_address().into(),
                amount: c.amount(),
            },
            stardust::UnlockCondition::Timelock(c) => Self::Timelock {
                milestone_index: c.milestone_index().0,
                timestamp: c.timestamp(),
            },
            stardust::UnlockCondition::Expiration(c) => Self::Expiration {
                return_address: c.return_address().into(),
                milestone_index: c.milestone_index().0,
                timestamp: c.timestamp(),
            },
            stardust::UnlockCondition::StateControllerAddress(a) => Self::StateControllerAddress(a.address().into()),
            stardust::UnlockCondition::GovernorAddress(a) => Self::GovernorAddress(a.address().into()),
            stardust::UnlockCondition::ImmutableAliasAddress(a) => {
                Self::ImmutableAliasAddress((*a.address().alias_id()).into())
            }
        }
    }
}

impl TryFrom<UnlockCondition> for stardust::UnlockCondition {
    type Error = crate::types::error::Error;

    fn try_from(value: UnlockCondition) -> Result<Self, Self::Error> {
        Ok(match value {
            UnlockCondition::Address(a) => Self::Address(stardust::AddressUnlockCondition::new(a.try_into()?)),
            UnlockCondition::StorageDepositReturn { return_address, amount } => Self::StorageDepositReturn(
                stardust::StorageDepositReturnUnlockCondition::new(return_address.try_into()?, amount)?,
            ),
            UnlockCondition::Timelock {
                milestone_index,
                timestamp,
            } => Self::Timelock(stardust::TimelockUnlockCondition::new(
                milestone_index.into(),
                timestamp,
            )?),
            UnlockCondition::Expiration {
                return_address,
                milestone_index,
                timestamp,
            } => Self::Expiration(stardust::ExpirationUnlockCondition::new(
                return_address.try_into()?,
                milestone_index.into(),
                timestamp,
            )?),
            UnlockCondition::StateControllerAddress(a) => {
                Self::StateControllerAddress(stardust::StateControllerAddressUnlockCondition::new(a.try_into()?))
            }
            UnlockCondition::GovernorAddress(a) => {
                Self::GovernorAddress(stardust::GovernorAddressUnlockCondition::new(a.try_into()?))
            }
            UnlockCondition::ImmutableAliasAddress(a) => {
                Self::ImmutableAliasAddress(stardust::ImmutableAliasAddressUnlockCondition::new(
                    bee_block_stardust::address::AliasAddress::new(a.try_into()?),
                ))
            }
        })
    }
}
