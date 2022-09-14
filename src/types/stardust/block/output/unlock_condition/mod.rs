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

#[cfg(all(test, feature = "rand"))]
mod test {
    use std::fmt::Debug;

    use mongodb::bson::{from_bson, to_bson};
    use serde::{de::DeserializeOwned, Serialize};

    use super::*;

    fn test<T>(t: T)
    where
        T: Serialize + DeserializeOwned + Debug + Eq,
    {
        let bson = to_bson(&t).unwrap();
        assert_eq!(t, from_bson::<T>(bson).unwrap());
    }

    #[test]
    fn test_unlock_condition_bson() {
        test(AddressUnlockCondition::rand());
        test(StorageDepositReturnUnlockCondition::rand());
        test(TimelockUnlockCondition::rand());
        test(ExpirationUnlockCondition::rand());
        test(GovernorAddressUnlockCondition::rand());
        test(StateControllerAddressUnlockCondition::rand());
        test(ImmutableAliasAddressUnlockCondition::rand());
    }
}
