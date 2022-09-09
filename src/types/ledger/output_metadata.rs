// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::types::{
    stardust::{
        block::{
            output::{Output, OutputId},
            payload::transaction::TransactionId,
            BlockId,
        },
        milestone::MilestoneTimestamp,
    },
    tangle::MilestoneIndex,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MilestoneIndexTimestamp {
    pub milestone_index: MilestoneIndex,
    pub milestone_timestamp: MilestoneTimestamp,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpentMetadata {
    pub transaction_id: TransactionId,
    pub spent: MilestoneIndexTimestamp,
}

/// Block metadata.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub block_id: BlockId,
    pub booked: MilestoneIndexTimestamp,
    pub spent_metadata: Option<SpentMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LedgerOutput {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub booked: MilestoneIndexTimestamp,
    pub output: Output,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LedgerSpent {
    pub output: LedgerOutput,
    pub spent_metadata: SpentMetadata,
}

#[cfg(feature = "inx")]
impl TryFrom<bee_inx::LedgerOutput> for LedgerOutput {
    type Error = bee_inx::Error;

    fn try_from(value: bee_inx::LedgerOutput) -> Result<Self, Self::Error> {
        Ok(Self {
            output: Into::into(&value.output.inner(&())?),
            output_id: value.output_id.into(),
            block_id: value.block_id.into(),
            booked: MilestoneIndexTimestamp {
                milestone_index: value.milestone_index_booked.into(),
                milestone_timestamp: value.milestone_timestamp_booked.into(),
            },
        })
    }
}

#[cfg(feature = "inx")]
impl TryFrom<bee_inx::LedgerSpent> for LedgerSpent {
    type Error = bee_inx::Error;

    fn try_from(value: bee_inx::LedgerSpent) -> Result<Self, Self::Error> {
        let output = LedgerOutput::try_from(value.output)?;

        Ok(Self {
            output,
            spent_metadata: SpentMetadata {
                transaction_id: value.transaction_id_spent.into(),
                spent: MilestoneIndexTimestamp {
                    milestone_index: value.milestone_index_spent.into(),
                    milestone_timestamp: value.milestone_timestamp_spent.into(),
                },
            },
        })
    }
}

/// The different number of bytes that are used for computing the rent cost.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RentStructureBytes {
    pub num_key_bytes: u64,
    pub num_data_bytes: u64,
}
