// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::slot::{SlotCommitment, SlotCommitmentId};
use serde::{Deserialize, Serialize};

use super::raw::Raw;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]

pub struct Commitment {
    pub commitment_id: SlotCommitmentId,
    pub commitment: Raw<SlotCommitment>,
}
