// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    payload::{milestone::MilestoneEssence, MilestoneOptions, MilestonePayload},
    rand,
};

use crate::signature::rand_signature;

pub fn rand_milestone_essence() -> MilestoneEssence {
    MilestoneEssence::new(
        1.into(),
        12345,
        rand::milestone::rand_milestone_id(),
        rand::parents::rand_parents(),
        rand::milestone::rand_merkle_root(),
        rand::milestone::rand_merkle_root(),
        "Foo".as_bytes().to_vec(),
        MilestoneOptions::new(vec![
            rand::milestone_option::rand_receipt_milestone_option().into(),
            rand::milestone_option::rand_receipt_milestone_option().into(),
            rand::milestone_option::rand_receipt_milestone_option().into(),
        ])
        .unwrap(),
    )
    .unwrap()
}

pub fn rand_milestone_payload() -> MilestonePayload {
    MilestonePayload::new(rand_milestone_essence(), vec![rand_signature()]).unwrap()
}
