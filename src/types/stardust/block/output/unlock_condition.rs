// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum UnlockCondition {
    Address {
        address: Address,
    },
    StorageDepositReturn {
        return_address: Address,
        #[serde(with = "crate::types::util::stringify")]
        amount: u64,
    },
    Timelock {
        timestamp: u32,
    },
    Expiration {
        return_address: Address,
        timestamp: u32,
    },
    StateControllerAddress {
        address: Address,
    },
    GovernorAddress {
        address: Address,
    },
    ImmutableAliasAddress {
        address: Address,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum UnlockConditionDescription {
    Address,
    StorageDepositReturn { amount: u64 },
    Timelock { timestamp: u32 },
    Expiration { timestamp: u32 },
    StateControllerAddress,
    GovernorAddress,
    ImmutableAliasAddress,
}

impl From<&UnlockCondition> for UnlockConditionDescription {
    fn from(value: &UnlockCondition) -> Self {
        match *value {
            UnlockCondition::Address { .. } => UnlockConditionDescription::Address,
            UnlockCondition::StorageDepositReturn { amount, .. } => {
                UnlockConditionDescription::StorageDepositReturn { amount }
            }
            UnlockCondition::Timelock { timestamp } => UnlockConditionDescription::Timelock { timestamp },
            UnlockCondition::Expiration { timestamp, .. } => UnlockConditionDescription::Expiration { timestamp },
            UnlockCondition::StateControllerAddress { .. } => UnlockConditionDescription::StateControllerAddress,
            UnlockCondition::GovernorAddress { .. } => UnlockConditionDescription::GovernorAddress,
            UnlockCondition::ImmutableAliasAddress { .. } => UnlockConditionDescription::ImmutableAliasAddress,
        }
    }
}

impl UnlockCondition {
    pub fn owning_address(&self) -> Option<(Address, UnlockConditionDescription)> {
        match *self {
            Self::Address { address } => Some((address, self.into())),
            Self::StorageDepositReturn { return_address, .. } => Some((return_address, self.into())),
            Self::Timelock { .. } => None,
            Self::Expiration { return_address, .. } => Some((return_address, self.into())),
            Self::StateControllerAddress { address } => Some((address, self.into())),
            Self::GovernorAddress { address } => Some((address, self.into())),
            Self::ImmutableAliasAddress { address } => Some((address, self.into())),
        }
    }
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
                timestamp: c.timestamp(),
            },
            bee::UnlockCondition::Expiration(c) => Self::Expiration {
                return_address: (*c.return_address()).into(),
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
    type Error = bee_block_stardust::Error;

    fn try_from(value: UnlockCondition) -> Result<Self, Self::Error> {
        Ok(match value {
            UnlockCondition::Address { address } => Self::Address(bee::AddressUnlockCondition::new(address.into())),
            UnlockCondition::StorageDepositReturn { return_address, amount } => Self::StorageDepositReturn(
                bee::StorageDepositReturnUnlockCondition::new(return_address.into(), amount)?,
            ),
            UnlockCondition::Timelock { timestamp } => Self::Timelock(bee::TimelockUnlockCondition::new(timestamp)?),
            UnlockCondition::Expiration {
                return_address,
                timestamp,
            } => Self::Expiration(bee::ExpirationUnlockCondition::new(return_address.into(), timestamp)?),
            UnlockCondition::StateControllerAddress { address } => {
                Self::StateControllerAddress(bee::StateControllerAddressUnlockCondition::new(address.into()))
            }
            UnlockCondition::GovernorAddress { address } => {
                Self::GovernorAddress(bee::GovernorAddressUnlockCondition::new(address.into()))
            }
            UnlockCondition::ImmutableAliasAddress { address } => {
                let bee_address = bee_block_stardust::address::Address::from(address);

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

        let block = get_test_timelock_condition(1);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<UnlockCondition>(bson).unwrap());

        let block = get_test_expiration_condition(bee_test::rand::address::rand_address().into(), 1);
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
        use bee_block_stardust::address::{Address as BeeAddress, AliasAddress as BeeAliasAddress};
        BeeAddress::from(BeeAliasAddress::new(bee_test::rand::output::rand_alias_id())).into()
    }

    pub(crate) fn get_test_address_condition(address: Address) -> UnlockCondition {
        UnlockCondition::Address { address }
    }

    pub(crate) fn get_test_storage_deposit_return_condition(return_address: Address, amount: u64) -> UnlockCondition {
        UnlockCondition::StorageDepositReturn { return_address, amount }
    }

    pub(crate) fn get_test_timelock_condition(timestamp: u32) -> UnlockCondition {
        UnlockCondition::Timelock { timestamp }
    }

    pub(crate) fn get_test_expiration_condition(return_address: Address, timestamp: u32) -> UnlockCondition {
        UnlockCondition::Expiration {
            return_address,
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
