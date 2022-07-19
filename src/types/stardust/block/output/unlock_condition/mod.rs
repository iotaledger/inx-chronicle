// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod address;
mod expiration;
mod governor_address;
mod immutable_alias_address;
mod state_controller_address;
mod storage_deposit_return;
mod timelock;

pub(crate) use self::{
    address::AddressUnlockCondition, expiration::ExpirationUnlockCondition,
    governor_address::GovernorAddressUnlockCondition, immutable_alias_address::ImmutableAliasAddressUnlockCondition,
    state_controller_address::StateControllerAddressUnlockCondition,
    storage_deposit_return::StorageDepositReturnUnlockCondition, timelock::TimelockUnlockCondition,
};
use super::OutputAmount;

#[cfg(test)]
pub(crate) mod test {

    use std::fmt::Debug;

    use bee_block_stardust::output::{unlock_condition as bee, Output};
    use bee_test::rand::{
        address::{rand_address, rand_alias_address},
        milestone::rand_milestone_index,
        number::{rand_number, rand_number_range},
        output::{rand_alias_id, unlock_condition::*},
    };
    use mongodb::bson::{from_bson, to_bson};
    use serde::{de::DeserializeOwned, Serialize};

    use super::*;

    fn test<U, T>(unlock: T)
    where
        for<'a> U: From<&'a T>,
        U: Serialize + DeserializeOwned + Debug + Eq,
    {
        let uc = U::from(&unlock);
        let bson = to_bson(&uc).unwrap();
        assert_eq!(uc, from_bson::<U>(bson).unwrap());
    }

    pub(crate) fn rand_expiration_unlock_condition() -> bee::ExpirationUnlockCondition {
        bee::ExpirationUnlockCondition::new(rand_address(), rand_number()).unwrap()
    }

    pub(crate) fn rand_storage_deposit_return_unlock_condition() -> bee::StorageDepositReturnUnlockCondition {
        bee::StorageDepositReturnUnlockCondition::new(rand_address(), rand_number_range(Output::AMOUNT_RANGE)).unwrap()
    }

    pub(crate) fn rand_timelock_unlock_condition() -> bee::TimelockUnlockCondition {
        bee::TimelockUnlockCondition::new(rand_milestone_index().0).unwrap()
    }

    pub(crate) fn rand_governor_address_unlock_condition() -> bee::GovernorAddressUnlockCondition {
        rand_governor_address_unlock_condition_different_from(&rand_alias_id())
    }

    pub(crate) fn rand_state_controller_address_unlock_condition() -> bee::StateControllerAddressUnlockCondition {
        rand_state_controller_address_unlock_condition_different_from(&rand_alias_id())
    }

    pub(crate) fn rand_immutable_alias_address_unlock_condition() -> bee::ImmutableAliasAddressUnlockCondition {
        bee::ImmutableAliasAddressUnlockCondition::new(rand_alias_address())
    }

    #[test]
    fn test_unlock_condition_bson() {
        test::<AddressUnlockCondition, bee::AddressUnlockCondition>(rand_address_unlock_condition());
        test::<StorageDepositReturnUnlockCondition, bee::StorageDepositReturnUnlockCondition>(
            rand_storage_deposit_return_unlock_condition(),
        );
        test::<TimelockUnlockCondition, bee::TimelockUnlockCondition>(
            bee::TimelockUnlockCondition::new(rand_number()).unwrap(),
        );
        test::<ExpirationUnlockCondition, bee::ExpirationUnlockCondition>(rand_expiration_unlock_condition());
        test::<GovernorAddressUnlockCondition, bee::GovernorAddressUnlockCondition>(
            rand_governor_address_unlock_condition(),
        );
        test::<StateControllerAddressUnlockCondition, bee::StateControllerAddressUnlockCondition>(
            rand_state_controller_address_unlock_condition(),
        );
        test::<ImmutableAliasAddressUnlockCondition, bee::ImmutableAliasAddressUnlockCondition>(
            rand_immutable_alias_address_unlock_condition(),
        );
    }
}
