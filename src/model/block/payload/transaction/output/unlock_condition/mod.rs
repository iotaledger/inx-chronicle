// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing unlock condition types.

pub mod address;
pub mod expiration;
pub mod governor_address;
pub mod immutable_alias_address;
pub mod state_controller_address;
pub mod storage_deposit_return;
pub mod timelock;

pub use self::{
    address::AddressUnlockCondition, expiration::ExpirationUnlockCondition,
    governor_address::GovernorAddressUnlockCondition, immutable_alias_address::ImmutableAliasAddressUnlockCondition,
    state_controller_address::StateControllerAddressUnlockCondition,
    storage_deposit_return::StorageDepositReturnUnlockCondition, timelock::TimelockUnlockCondition,
};
use super::TokenAmount;

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_address_unlock_bson() {
        let unlock = AddressUnlockCondition::rand();
        let bson = to_bson(&unlock).unwrap();
        from_bson::<AddressUnlockCondition>(bson).unwrap();
    }

    #[test]
    fn test_storage_deposit_unlock_bson() {
        let ctx = iota_sdk::types::block::protocol::protocol_parameters();
        let unlock = StorageDepositReturnUnlockCondition::rand(&ctx);
        let bson = to_bson(&unlock).unwrap();
        from_bson::<StorageDepositReturnUnlockCondition>(bson).unwrap();
    }

    #[test]
    fn test_timelock_unlock_bson() {
        let unlock = TimelockUnlockCondition::rand();
        let bson = to_bson(&unlock).unwrap();
        from_bson::<TimelockUnlockCondition>(bson).unwrap();
    }

    #[test]
    fn test_expiration_unlock_bson() {
        let unlock = ExpirationUnlockCondition::rand();
        let bson = to_bson(&unlock).unwrap();
        from_bson::<ExpirationUnlockCondition>(bson).unwrap();
    }

    #[test]
    fn test_governor_unlock_bson() {
        let unlock = GovernorAddressUnlockCondition::rand();
        let bson = to_bson(&unlock).unwrap();
        from_bson::<GovernorAddressUnlockCondition>(bson).unwrap();
    }

    #[test]
    fn test_state_controller_unlock_bson() {
        let unlock = StateControllerAddressUnlockCondition::rand();
        let bson = to_bson(&unlock).unwrap();
        from_bson::<StateControllerAddressUnlockCondition>(bson).unwrap();
    }

    #[test]
    fn test_immut_alias_unlock_bson() {
        let unlock = ImmutableAliasAddressUnlockCondition::rand();
        let bson = to_bson(&unlock).unwrap();
        from_bson::<ImmutableAliasAddressUnlockCondition>(bson).unwrap();
    }
}
