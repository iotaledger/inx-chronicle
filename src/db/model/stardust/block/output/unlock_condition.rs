// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::db::model::stardust::block::Address;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UnlockCondition {
    #[serde(rename = "address")]
    Address { address: Address },
    #[serde(rename = "storage_deposit_return")]
    StorageDepositReturn {
        return_address: Address,
        #[serde(with = "crate::db::model::util::stringify")]
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
    ImmutableAliasAddress { address: Address },
}

impl From<&bee::UnlockCondition> for UnlockCondition {
    fn from(value: &bee::UnlockCondition) -> Self {
        match value {
            bee::UnlockCondition::Address(a) => Self::Address {
                address: (*a.address()).into(),
            },
            bee::UnlockCondition::StorageDepositReturn(c) => Self::StorageDepositReturn {
                return_address: (*c.return_address()).into(),
                amount: c.amount(),
            },
            bee::UnlockCondition::Timelock(c) => Self::Timelock {
                milestone_index: c.milestone_index().0,
                timestamp: c.timestamp(),
            },
            bee::UnlockCondition::Expiration(c) => Self::Expiration {
                return_address: (*c.return_address()).into(),
                milestone_index: c.milestone_index().0,
                timestamp: c.timestamp(),
            },
            bee::UnlockCondition::StateControllerAddress(a) => Self::StateControllerAddress {
                address: (*a.address()).into(),
            },
            bee::UnlockCondition::GovernorAddress(a) => Self::GovernorAddress {
                address: (*a.address()).into(),
            },
            bee::UnlockCondition::ImmutableAliasAddress(a) => Self::ImmutableAliasAddress {
                address: (*a.address()).into(),
            },
        }
    }
}

impl TryFrom<UnlockCondition> for bee::UnlockCondition {
    type Error = crate::db::error::Error;

    fn try_from(value: UnlockCondition) -> Result<Self, Self::Error> {
        Ok(match value {
            UnlockCondition::Address { address } => {
                Self::Address(bee::AddressUnlockCondition::new(address.try_into()?))
            }
            UnlockCondition::StorageDepositReturn { return_address, amount } => Self::StorageDepositReturn(
                bee::StorageDepositReturnUnlockCondition::new(return_address.try_into()?, amount)?,
            ),
            UnlockCondition::Timelock {
                milestone_index,
                timestamp,
            } => Self::Timelock(bee::TimelockUnlockCondition::new(milestone_index.into(), timestamp)?),
            UnlockCondition::Expiration {
                return_address,
                milestone_index,
                timestamp,
            } => Self::Expiration(bee::ExpirationUnlockCondition::new(
                return_address.try_into()?,
                milestone_index.into(),
                timestamp,
            )?),
            UnlockCondition::StateControllerAddress { address } => {
                Self::StateControllerAddress(bee::StateControllerAddressUnlockCondition::new(address.try_into()?))
            }
            UnlockCondition::GovernorAddress { address } => {
                Self::GovernorAddress(bee::GovernorAddressUnlockCondition::new(address.try_into()?))
            }
            UnlockCondition::ImmutableAliasAddress { address } => {
                let bee_address = bee_block_stardust::address::Address::try_from(address)?;

                if let bee_block_stardust::address::Address::Alias(alias_address) = bee_address {
                    Self::ImmutableAliasAddress(bee::ImmutableAliasAddressUnlockCondition::new(alias_address))
                } else {
                    Err(bee_block_stardust::Error::InvalidAddressKind(bee_address.kind()))?
                }
            }
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_unlock_condition_bson() {
        let block = get_test_address_condition(bee_test::rand::address::rand_address().into());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_storage_deposit_return_condition(bee_test::rand::address::rand_address().into(), 1);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_timelock_condition(1, 1);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_expiration_condition(bee_test::rand::address::rand_address().into(), 1, 1);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_state_controller_address_condition(bee_test::rand::address::rand_address().into());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_governor_address_condition(bee_test::rand::address::rand_address().into());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_immut_alias_address_condition(get_test_alias_address_as_address());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());
    }

    pub(crate) fn get_test_alias_address_as_address() -> Address {
        bee_block_stardust::address::Address::from(bee_block_stardust::address::AliasAddress::new(
            bee_test::rand::output::rand_alias_id(),
        ))
        .into()
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

    pub(crate) fn get_test_immut_alias_address_condition(address: Address) -> UnlockCondition {
        UnlockCondition::ImmutableAliasAddress { address }
    }
}
