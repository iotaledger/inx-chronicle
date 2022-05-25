// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::types::{
    stardust::{
        block::{BlockId, TransactionId, OutputId},
        milestone::MilestoneTimestamp,
    },
    tangle::MilestoneIndex,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpentMetadata {
    pub transaction_id: TransactionId,
    pub milestone_index_spent: MilestoneIndex,
    pub milestone_timestamp_spent: MilestoneTimestamp,
}

/// Block metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub transaction_id: TransactionId,
    pub milestone_index_booked: MilestoneIndex,
    pub milestone_timestamp_booked: MilestoneTimestamp,
    pub spent: Option<SpentMetadata>,
}
