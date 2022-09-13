// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    output::{
        unlock_condition::{
            ExpirationUnlockCondition, ImmutableAliasAddressUnlockCondition, StorageDepositReturnUnlockCondition,
            TimelockUnlockCondition,
        },
        Output,
    },
    rand::{
        address::{rand_address, rand_alias_address},
        milestone::rand_milestone_index,
        number::{rand_number, rand_number_range},
    },
};

pub fn rand_expiration_unlock_condition() -> ExpirationUnlockCondition {
    ExpirationUnlockCondition::new(rand_address(), rand_number()).unwrap()
}

pub fn rand_storage_deposit_return_unlock_condition() -> StorageDepositReturnUnlockCondition {
    StorageDepositReturnUnlockCondition::new(rand_address(), rand_number_range(Output::AMOUNT_RANGE)).unwrap()
}

pub fn rand_timelock_unlock_condition() -> TimelockUnlockCondition {
    TimelockUnlockCondition::new(rand_milestone_index().0).unwrap()
}

pub fn rand_immutable_alias_address_unlock_condition() -> ImmutableAliasAddressUnlockCondition {
    ImmutableAliasAddressUnlockCondition::new(rand_alias_address())
}
