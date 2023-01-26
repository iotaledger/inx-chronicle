// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use crate::types::{
    stardust::{
        block::{
            output::{Output, OutputAmount, OutputId},
            payload::transaction::TransactionId,
            Address, BlockId,
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
    pub rent_structure: RentStructureBytes,
}

impl LedgerOutput {
    pub fn amount(&self) -> OutputAmount {
        self.output.amount()
    }

    pub fn owning_address(&self) -> Option<&Address> {
        self.output.owning_address()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LedgerSpent {
    pub output: LedgerOutput,
    pub spent_metadata: SpentMetadata,
}

impl LedgerSpent {
    pub fn amount(&self) -> OutputAmount {
        self.output.amount()
    }

    pub fn owning_address(&self) -> Option<&Address> {
        self.output.owning_address()
    }
}

/// The different number of bytes that are used for computing the rent cost.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RentStructureBytes {
    /// The number of key bytes in an output.
    pub num_key_bytes: u64,
    /// The number of data bytes in an output.
    pub num_data_bytes: u64,
}

impl RentStructureBytes {
    pub fn compute(output: &iota_types::block::output::Output) -> Self {
        use iota_types::block::output::{Rent, RentStructureBuilder};

        let rent_cost = |byte_cost, data_factor, key_factor| {
            output.rent_cost(
                &RentStructureBuilder::new()
                    .byte_cost(byte_cost)
                    .byte_factor_data(data_factor)
                    .byte_factor_key(key_factor)
                    .finish(),
            )
        };

        RentStructureBytes {
            num_data_bytes: rent_cost(1, 1, 0),
            num_key_bytes: rent_cost(1, 0, 1),
        }
    }

    pub fn rent_cost(&self, config: &iota_types::block::output::RentStructure) -> u64 {
        (self.num_data_bytes * config.byte_factor_data() as u64 + self.num_key_bytes * config.byte_factor_key() as u64)
            * config.byte_cost() as u64
    }
}

#[cfg(feature = "inx")]
mod inx {
    use packable::PackableExt;

    use super::*;
    use crate::{inx::InxError, maybe_missing};

    #[cfg(feature = "inx")]
    impl TryFrom<::inx::proto::LedgerOutput> for LedgerOutput {
        type Error = InxError;

        fn try_from(value: ::inx::proto::LedgerOutput) -> Result<Self, Self::Error> {
            let data = maybe_missing!(value.output).data;
            let bee_output = iota_types::block::output::Output::unpack_unverified(data)
                .map_err(|e| InxError::InvalidRawBytes(format!("{e:?}")))?;

            Ok(Self {
                rent_structure: RentStructureBytes::compute(&bee_output),
                output: Into::into(&bee_output),
                output_id: maybe_missing!(value.output_id).try_into()?,
                block_id: maybe_missing!(value.block_id).try_into()?,
                booked: MilestoneIndexTimestamp {
                    milestone_index: value.milestone_index_booked.into(),
                    milestone_timestamp: value.milestone_timestamp_booked.into(),
                },
            })
        }
    }

    #[cfg(feature = "inx")]
    impl TryFrom<::inx::proto::LedgerSpent> for LedgerSpent {
        type Error = InxError;

        fn try_from(value: ::inx::proto::LedgerSpent) -> Result<Self, Self::Error> {
            let output = LedgerOutput::try_from(maybe_missing!(value.output))?;

            Ok(Self {
                output,
                spent_metadata: SpentMetadata {
                    transaction_id: maybe_missing!(value.transaction_id_spent).try_into()?,
                    spent: MilestoneIndexTimestamp {
                        milestone_index: value.milestone_index_spent.into(),
                        milestone_timestamp: value.milestone_timestamp_spent.into(),
                    },
                },
            })
        }
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "rand")]
    #[test]
    fn test_compute_rent_structure() {
        use iota_types::block::{output::Rent, rand::output};

        use super::RentStructureBytes;

        let protocol_params = iota_types::block::protocol::protocol_parameters();

        let outputs = [
            output::rand_basic_output(protocol_params.token_supply()).into(),
            output::rand_alias_output(protocol_params.token_supply()).into(),
            output::rand_foundry_output(protocol_params.token_supply()).into(),
            output::rand_nft_output(protocol_params.token_supply()).into(),
        ];

        for output in outputs {
            let rent = RentStructureBytes::compute(&output);
            assert_eq!(
                rent.rent_cost(protocol_params.rent_structure()),
                output.rent_cost(protocol_params.rent_structure())
            );
        }
    }
}
