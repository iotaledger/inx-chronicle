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
    address::AddressUnlockConditionDto, expiration::ExpirationUnlockConditionDto,
    governor_address::GovernorAddressUnlockConditionDto,
    immutable_alias_address::ImmutableAccountAddressUnlockConditionDto,
    state_controller_address::StateControllerAddressUnlockConditionDto,
    storage_deposit_return::StorageDepositReturnUnlockConditionDto, timelock::TimelockUnlockConditionDto,
};

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};

//     use super::*;

//     #[test]
//     fn test_address_unlock_bson() {
//         let unlock = AddressUnlockCondition::rand();
//         let bson = to_bson(&unlock).unwrap();
//         from_bson::<AddressUnlockCondition>(bson).unwrap();
//     }

//     #[test]
//     fn test_storage_deposit_unlock_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let unlock = StorageDepositReturnUnlockConditionDto::rand(&ctx);
//         let bson = to_bson(&unlock).unwrap();
//         from_bson::<StorageDepositReturnUnlockConditionDto>(bson).unwrap();
//     }

//     #[test]
//     fn test_timelock_unlock_bson() {
//         let unlock = TimelockUnlockConditionDto::rand();
//         let bson = to_bson(&unlock).unwrap();
//         from_bson::<TimelockUnlockConditionDto>(bson).unwrap();
//     }

//     #[test]
//     fn test_expiration_unlock_bson() {
//         let unlock = ExpirationUnlockConditionDto::rand();
//         let bson = to_bson(&unlock).unwrap();
//         from_bson::<ExpirationUnlockConditionDto>(bson).unwrap();
//     }

//     #[test]
//     fn test_governor_unlock_bson() {
//         let unlock = GovernorAddressUnlockConditionDto::rand();
//         let bson = to_bson(&unlock).unwrap();
//         from_bson::<GovernorAddressUnlockConditionDto>(bson).unwrap();
//     }

//     #[test]
//     fn test_state_controller_unlock_bson() {
//         let unlock = StateControllerAddressUnlockConditionDto::rand();
//         let bson = to_bson(&unlock).unwrap();
//         from_bson::<StateControllerAddressUnlockConditionDto>(bson).unwrap();
//     }

//     #[test]
//     fn test_immut_alias_unlock_bson() {
//         let unlock = ImmutableAliasAddressUnlockConditionDto::rand();
//         let bson = to_bson(&unlock).unwrap();
//         from_bson::<ImmutableAliasAddressUnlockConditionDto>(bson).unwrap();
//     }
// }
