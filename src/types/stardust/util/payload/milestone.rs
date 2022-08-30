// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::payload::milestone as bee;

use crate::types::stardust::{
    block::payload::{
        milestone::{MigratedFundsEntry, MilestoneEssence},
        MilestonePayload,
    },
    util::signature::get_test_signature,
};

const TAIL_TRANSACTION_HASH1: [u8; 49] = [
    222, 235, 107, 67, 2, 173, 253, 93, 165, 90, 166, 45, 102, 91, 19, 137, 71, 146, 156, 180, 248, 31, 56, 25, 68,
    154, 98, 100, 64, 108, 203, 48, 76, 75, 114, 150, 34, 153, 203, 35, 225, 120, 194, 175, 169, 207, 80, 229, 10,
];
const TAIL_TRANSACTION_HASH2: [u8; 49] = [
    222, 235, 107, 67, 2, 173, 253, 93, 165, 90, 166, 45, 102, 91, 19, 137, 71, 146, 156, 180, 248, 31, 56, 25, 68,
    154, 98, 100, 64, 108, 203, 48, 76, 75, 114, 150, 34, 153, 203, 35, 225, 120, 194, 175, 169, 207, 80, 229, 11,
];
const TAIL_TRANSACTION_HASH3: [u8; 49] = [
    222, 235, 107, 67, 2, 173, 253, 93, 165, 90, 166, 45, 102, 91, 19, 137, 71, 146, 156, 180, 248, 31, 56, 25, 68,
    154, 98, 100, 64, 108, 203, 48, 76, 75, 114, 150, 34, 153, 203, 35, 225, 120, 194, 175, 169, 207, 80, 229, 12,
];

pub fn get_test_ed25519_migrated_funds_entry() -> MigratedFundsEntry {
    MigratedFundsEntry::from(
        &bee::option::MigratedFundsEntry::new(
            bee::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH1).unwrap(),
            bee_block_stardust::address::Address::Ed25519(bee_block_stardust::rand::address::rand_ed25519_address()),
            2000000,
        )
        .unwrap(),
    )
}

pub fn get_test_alias_migrated_funds_entry() -> MigratedFundsEntry {
    MigratedFundsEntry::from(
        &bee::option::MigratedFundsEntry::new(
            bee::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH2).unwrap(),
            bee_block_stardust::address::Address::Alias(bee_block_stardust::rand::address::rand_alias_address()),
            2000000,
        )
        .unwrap(),
    )
}

pub fn get_test_nft_migrated_funds_entry() -> MigratedFundsEntry {
    MigratedFundsEntry::from(
        &bee::option::MigratedFundsEntry::new(
            bee::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH3).unwrap(),
            bee_block_stardust::address::Address::Nft(bee_block_stardust::rand::address::rand_nft_address()),
            2000000,
        )
        .unwrap(),
    )
}

pub fn get_test_milestone_essence() -> MilestoneEssence {
    MilestoneEssence::from(
        &bee::MilestoneEssence::new(
            1.into(),
            12345,
            bee_block_stardust::rand::milestone::rand_milestone_id(),
            bee_block_stardust::rand::parents::rand_parents(),
            bee_block_stardust::rand::milestone::rand_merkle_root(),
            bee_block_stardust::rand::milestone::rand_merkle_root(),
            "Foo".as_bytes().to_vec(),
            bee::MilestoneOptions::new(vec![bee::option::MilestoneOption::Receipt(
                bee::option::ReceiptMilestoneOption::new(
                    1.into(),
                    false,
                    vec![
                        get_test_ed25519_migrated_funds_entry().try_into().unwrap(),
                        get_test_alias_migrated_funds_entry().try_into().unwrap(),
                        get_test_nft_migrated_funds_entry().try_into().unwrap(),
                    ],
                    bee_block_stardust::payload::TreasuryTransactionPayload::new(
                        bee_block_stardust::rand::input::rand_treasury_input(),
                        bee_block_stardust::rand::output::rand_treasury_output(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            )])
            .unwrap(),
        )
        .unwrap(),
    )
}

pub fn get_test_milestone_payload() -> MilestonePayload {
    MilestonePayload::from(
        &bee::MilestonePayload::new(
            get_test_milestone_essence().try_into().unwrap(),
            vec![get_test_signature().try_into().unwrap()],
        )
        .unwrap(),
    )
}
