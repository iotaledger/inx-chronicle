// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::types::{
    stardust::{
        block::{BlockId, TransactionId},
        milestone::MilestoneTimestamp,
    },
    tangle::MilestoneIndex,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpentMetadata {
    transaction_id: TransactionId,
    milestone_index_spent: MilestoneIndex,
    milestone_timestamp_spent: MilestoneTimestamp,
}

/// Block metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputMetadata {
    block_id: BlockId,
    milestone_index_booked: MilestoneIndex,
    milestone_timestamp_booked: MilestoneTimestamp,
    spent: Option<SpentMetadata>,
}
