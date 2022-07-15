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
    pub booked: MilestoneIndexTimestamp,
    pub spent: Option<SpentMetadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputWithMetadata {
    pub output: Output,
    pub metadata: OutputMetadata,
}

#[cfg(feature = "inx")]
impl From<inx::LedgerOutput> for OutputWithMetadata {
    fn from(value: inx::LedgerOutput) -> Self {
        let output_id = OutputId::from(value.output_id);
        Self {
            output: Into::into(&value.output),
            metadata: OutputMetadata {
                output_id,
                block_id: value.block_id.into(),
                booked: MilestoneIndexTimestamp {
                    milestone_index: value.milestone_index_booked.into(),
                    milestone_timestamp: value.milestone_timestamp_booked.into(),
                },
                spent: None,
            },
        }
    }
}

#[cfg(feature = "inx")]
impl From<inx::LedgerSpent> for OutputWithMetadata {
    fn from(value: inx::LedgerSpent) -> Self {
        let mut delta = OutputWithMetadata::from(value.output);

        delta.metadata.spent.replace(SpentMetadata {
            transaction_id: value.transaction_id_spent.into(),
            spent: MilestoneIndexTimestamp {
                milestone_index: value.milestone_index_spent.into(),
                milestone_timestamp: value.milestone_timestamp_spent.into(),
            },
        });

        delta
    }
}
