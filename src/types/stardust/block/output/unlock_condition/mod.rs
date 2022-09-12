// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod address;
mod expiration;
mod governor_address;
mod immutable_alias_address;
mod state_controller_address;
mod storage_deposit_return;
mod timelock;

pub use self::{
    address::AddressUnlockCondition, expiration::ExpirationUnlockCondition,
    governor_address::GovernorAddressUnlockCondition, immutable_alias_address::ImmutableAliasAddressUnlockCondition,
    state_controller_address::StateControllerAddressUnlockCondition,
    storage_deposit_return::StorageDepositReturnUnlockCondition, timelock::TimelockUnlockCondition,
};
use super::OutputAmount;

#[cfg(test)]
mod test {

    use std::fmt::Debug;

    use bee_block_stardust::{
        output::unlock_condition as bee,
        rand::{number::rand_number, output::unlock_condition::*},
    };
    use mongodb::bson::{from_bson, to_bson};
    use serde::{de::DeserializeOwned, Serialize};

    use super::*;
    use crate::types::stardust::util::output::unlock_condition::*;

    fn test<U, T>(unlock: T)
    where
        for<'a> U: From<&'a T>,
        U: Serialize + DeserializeOwned + Debug + Eq,
    {
        let uc = U::from(&unlock);
        let bson = to_bson(&uc).unwrap();
        assert_eq!(uc, from_bson::<U>(bson).unwrap());
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
