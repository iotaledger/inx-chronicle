// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use bee_block_stardust::{
    payload::{milestone::MilestoneEssence, MilestoneOptions, MilestonePayload},
    rand::{
        milestone::{rand_merkle_root, rand_milestone_id},
        milestone_option::rand_receipt_milestone_option,
        parents::rand_parents,
    },
};

use crate::signature::rand_signature;

/// Generates a random [`MilestoneEssence`].
pub fn rand_milestone_essence(index: u32) -> MilestoneEssence {
    MilestoneEssence::new(
        index.into(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as _,
        rand_milestone_id(),
        rand_parents(),
        rand_merkle_root(),
        rand_merkle_root(),
        "Foo".as_bytes().to_vec(),
        MilestoneOptions::new(vec![rand_receipt_milestone_option().into()]).unwrap(),
    )
    .unwrap()
}

/// Generates a random [`MilestonePayload`].
pub fn rand_milestone_payload(index: u32) -> MilestonePayload {
    MilestonePayload::new(rand_milestone_essence(index), vec![rand_signature()]).unwrap()
}
