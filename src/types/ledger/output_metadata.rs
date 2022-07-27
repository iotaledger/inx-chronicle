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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spent_metadata: Option<SpentMetadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputWithMetadata {
    pub output: Output,
    pub metadata: OutputMetadata,
}

#[cfg(feature = "inx")]
impl TryFrom<bee_inx::LedgerOutput> for OutputWithMetadata {
    type Error = bee_inx::Error;

    fn try_from(value: bee_inx::LedgerOutput) -> Result<Self, Self::Error> {
        Ok(Self {
            output: Into::into(&value.output.inner()?),
            metadata: OutputMetadata {
                output_id: value.output_id.into(),
                block_id: value.block_id.into(),
                booked: MilestoneIndexTimestamp {
                    milestone_index: value.milestone_index_booked.into(),
                    milestone_timestamp: value.milestone_timestamp_booked.into(),
                },
                spent_metadata: None,
            },
        })
    }
}

#[cfg(feature = "inx")]
impl TryFrom<bee_inx::LedgerSpent> for OutputWithMetadata {
    type Error = bee_inx::Error;

    fn try_from(value: bee_inx::LedgerSpent) -> Result<Self, Self::Error> {
        let mut delta = OutputWithMetadata::try_from(value.output)?;

        delta.metadata.spent_metadata.replace(SpentMetadata {
            transaction_id: value.transaction_id_spent.into(),
            spent: MilestoneIndexTimestamp {
                milestone_index: value.milestone_index_spent.into(),
                milestone_timestamp: value.milestone_timestamp_spent.into(),
            },
        });

        Ok(delta)
    }
}

/// The different number of bytes that are used for computing the rent cost.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RentStructureBytes {
    pub num_key_bytes: u64,
    pub num_data_bytes: u64,
}
