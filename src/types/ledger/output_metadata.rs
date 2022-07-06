// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::types::{
    stardust::{
        block::{BlockId, Output, OutputId, TransactionId},
        milestone::MilestoneTimestamp,
    },
    tangle::MilestoneIndex,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MilestoneIndexTimestamp {
    pub milestone_index: MilestoneIndex,
    pub milestone_timestamp: MilestoneTimestamp,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct SpentMetadata {
    pub transaction_id: TransactionId,
    pub spent: MilestoneIndexTimestamp,
}

/// Block metadata.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub transaction_id: TransactionId,
    pub booked: MilestoneIndexTimestamp,
    pub spent: Option<SpentMetadata>,
    pub latest_milestone: MilestoneIndexTimestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputWithMetadata {
    pub output: Output,
    pub metadata: OutputMetadata,
}

pub struct LedgerUpdate {
    pub output_id: OutputId,
    pub output: Output,
    pub block_id: BlockId,
    pub booked: MilestoneIndexTimestamp,
    pub spent: Option<MilestoneIndexTimestamp>,
}

#[cfg(feature = "inx")]
impl From<inx::LedgerOutput> for LedgerUpdate {
    fn from(value: inx::LedgerOutput) -> Self {
        let output_id = OutputId::from(value.output_id);
        Self {
            output_id,
            output: (&value.output).into(),
            block_id: value.block_id.into(),
            booked: MilestoneIndexTimestamp {
                milestone_index: value.milestone_index_booked.into(),
                milestone_timestamp: value.milestone_timestamp_booked.into(),
            },
            spent: None,
        }
    }
}

#[cfg(feature = "inx")]
impl From<inx::LedgerSpent> for LedgerUpdate {
    fn from(value: inx::LedgerSpent) -> Self {
        let mut delta = LedgerUpdate::from(value.output);

        delta.spent.replace(MilestoneIndexTimestamp {
            milestone_index: value.milestone_index_spent.into(),
            milestone_timestamp: value.milestone_timestamp_spent.into(),
        });

        delta
    }
}
