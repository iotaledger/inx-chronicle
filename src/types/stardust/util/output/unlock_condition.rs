// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    output::unlock_condition as bee,
    rand::{
        address::{rand_address, rand_alias_address},
        milestone::rand_milestone_index,
        number::{rand_number, rand_number_range},
        output::{
            rand_alias_id,
            unlock_condition::{
                rand_governor_address_unlock_condition_different_from,
                rand_state_controller_address_unlock_condition_different_from,
            },
        },
    },
};

pub fn rand_expiration_unlock_condition() -> bee::ExpirationUnlockCondition {
    bee::ExpirationUnlockCondition::new(rand_address(), rand_number()).unwrap()
}

pub fn rand_storage_deposit_return_unlock_condition() -> bee::StorageDepositReturnUnlockCondition {
    bee::StorageDepositReturnUnlockCondition::new(
        rand_address(),
        rand_number_range(bee_block_stardust::output::Output::AMOUNT_RANGE),
    )
    .unwrap()
}

pub fn rand_timelock_unlock_condition() -> bee::TimelockUnlockCondition {
    bee::TimelockUnlockCondition::new(rand_milestone_index().0).unwrap()
}

pub fn rand_governor_address_unlock_condition() -> bee::GovernorAddressUnlockCondition {
    rand_governor_address_unlock_condition_different_from(&rand_alias_id())
}

pub fn rand_state_controller_address_unlock_condition() -> bee::StateControllerAddressUnlockCondition {
    rand_state_controller_address_unlock_condition_different_from(&rand_alias_id())
}

pub fn rand_immutable_alias_address_unlock_condition() -> bee::ImmutableAliasAddressUnlockCondition {
    bee::ImmutableAliasAddressUnlockCondition::new(rand_alias_address())
}
