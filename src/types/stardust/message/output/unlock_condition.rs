// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output::unlock_condition as stardust;
use serde::{Deserialize, Serialize};

use super::AliasId;
use crate::types::stardust::message::Address;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UnlockCondition {
    #[serde(rename = "address")]
    Address { address: Address },
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
    StateControllerAddress { address: Address },
    #[serde(rename = "governor_address")]
    GovernorAddress { address: Address },
    #[serde(rename = "immutable_alias_address")]
    ImmutableAliasAddress { alias_id: AliasId },
}

impl From<&stardust::UnlockCondition> for UnlockCondition {
    fn from(value: &stardust::UnlockCondition) -> Self {
        match value {
            stardust::UnlockCondition::Address(a) => Self::Address {
                address: (*a.address()).into(),
            },
            stardust::UnlockCondition::StorageDepositReturn(c) => Self::StorageDepositReturn {
                return_address: (*c.return_address()).into(),
                amount: c.amount(),
            },
            stardust::UnlockCondition::Timelock(c) => Self::Timelock {
                milestone_index: c.milestone_index().0,
                timestamp: c.timestamp(),
            },
            stardust::UnlockCondition::Expiration(c) => Self::Expiration {
                return_address: (*c.return_address()).into(),
                milestone_index: c.milestone_index().0,
                timestamp: c.timestamp(),
            },
            stardust::UnlockCondition::StateControllerAddress(a) => Self::StateControllerAddress {
                address: (*a.address()).into(),
            },
            stardust::UnlockCondition::GovernorAddress(a) => Self::GovernorAddress {
                address: (*a.address()).into(),
            },
            stardust::UnlockCondition::ImmutableAliasAddress(a) => Self::ImmutableAliasAddress {
                alias_id: (*a.address().alias_id()).into(),
            },
        }
    }
}

impl TryFrom<UnlockCondition> for stardust::UnlockCondition {
    type Error = crate::types::error::Error;

    fn try_from(value: UnlockCondition) -> Result<Self, Self::Error> {
        Ok(match value {
            UnlockCondition::Address { address } => {
                Self::Address(stardust::AddressUnlockCondition::new(address.try_into()?))
            }
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
            UnlockCondition::StateControllerAddress { address } => Self::StateControllerAddress(
                stardust::StateControllerAddressUnlockCondition::new(address.try_into()?),
            ),
            UnlockCondition::GovernorAddress { address } => {
                Self::GovernorAddress(stardust::GovernorAddressUnlockCondition::new(address.try_into()?))
            }
            UnlockCondition::ImmutableAliasAddress { alias_id } => {
                Self::ImmutableAliasAddress(stardust::ImmutableAliasAddressUnlockCondition::new(
                    bee_message_stardust::address::AliasAddress::new(alias_id.try_into()?),
                ))
            }
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::{
        address::test::{get_test_alias_address, get_test_ed25519_address, get_test_nft_address},
        output::alias::test::get_test_alias_id,
    };

    #[test]
    fn test_unlock_condition_bson() {
        let block = get_test_address_condition(get_test_ed25519_address());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_address_condition(get_test_alias_address());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_storage_deposit_return_condition(get_test_ed25519_address(), 1);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_storage_deposit_return_condition(get_test_nft_address(), 1);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_timelock_condition(1, 1);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_expiration_condition(get_test_ed25519_address(), 1, 1);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_state_controller_address_condition(get_test_ed25519_address());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_governor_address_condition(get_test_alias_address());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_immut_alias_address_condition(get_test_alias_id());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());
    }

    pub(crate) fn get_test_address_condition(address: Address) -> UnlockCondition {
        UnlockCondition::Address { address }
    }

    pub(crate) fn get_test_storage_deposit_return_condition(return_address: Address, amount: u64) -> UnlockCondition {
        UnlockCondition::StorageDepositReturn { return_address, amount }
    }

    pub(crate) fn get_test_timelock_condition(milestone_index: u32, timestamp: u32) -> UnlockCondition {
        UnlockCondition::Timelock {
            milestone_index,
            timestamp,
        }
    }

    pub(crate) fn get_test_expiration_condition(
        return_address: Address,
        milestone_index: u32,
        timestamp: u32,
    ) -> UnlockCondition {
        UnlockCondition::Expiration {
            return_address,
            milestone_index,
            timestamp,
        }
    }

    pub(crate) fn get_test_state_controller_address_condition(address: Address) -> UnlockCondition {
        UnlockCondition::StateControllerAddress { address }
    }

    pub(crate) fn get_test_governor_address_condition(address: Address) -> UnlockCondition {
        UnlockCondition::GovernorAddress { address }
    }

    pub(crate) fn get_test_immut_alias_address_condition(alias_id: AliasId) -> UnlockCondition {
        UnlockCondition::ImmutableAliasAddress { alias_id }
    }
}
