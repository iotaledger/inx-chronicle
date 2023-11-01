// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Ledger output types

use serde::{Deserialize, Serialize};

use super::{Output, OutputId, TokenAmount};
use crate::model::{block::BlockId, metadata::SpentMetadata, tangle::MilestoneIndexTimestamp, utxo::Address};

/// An unspent output according to the ledger.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerOutput {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub booked: MilestoneIndexTimestamp,
    pub output: Output,
    pub rent_structure: RentStructureBytes,
}

#[allow(missing_docs)]
impl LedgerOutput {
    pub fn output_id(&self) -> OutputId {
        self.output_id
    }

    pub fn amount(&self) -> TokenAmount {
        self.output.amount()
    }

    pub fn owning_address(&self) -> Option<&Address> {
        self.output.owning_address()
    }
}

/// A spent output according to the ledger.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerSpent {
    pub output: LedgerOutput,
    pub spent_metadata: SpentMetadata,
}

#[allow(missing_docs)]
impl LedgerSpent {
    pub fn output_id(&self) -> OutputId {
        self.output.output_id
    }

    pub fn amount(&self) -> TokenAmount {
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
    #[allow(missing_docs)]
    pub fn compute(output: &iota_sdk::types::block::output::Output) -> Self {
        use iota_sdk::types::block::output::{Rent, RentStructure};

        let rent_cost = |byte_cost, data_factor, key_factor| {
            output.rent_cost(
                &RentStructure::default()
                    .with_byte_cost(byte_cost)
                    .with_byte_factor_data(data_factor)
                    .with_byte_factor_key(key_factor),
            )
        };

        RentStructureBytes {
            num_data_bytes: rent_cost(1, 1, 0),
            num_key_bytes: rent_cost(1, 0, 1),
        }
    }
}

#[cfg(feature = "inx")]
mod inx {
    use packable::PackableExt;

    use super::*;
    use crate::{inx::InxError, maybe_missing};

    impl TryFrom<::inx::proto::LedgerOutput> for LedgerOutput {
        type Error = InxError;

        fn try_from(value: ::inx::proto::LedgerOutput) -> Result<Self, Self::Error> {
            let data = maybe_missing!(value.output).data;
            let bee_output = iota_sdk::types::block::output::Output::unpack_unverified(data)
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
    impl super::RentStructureBytes {
        fn rent_cost(&self, config: &iota_sdk::types::block::output::RentStructure) -> u64 {
            (self.num_data_bytes * config.byte_factor_data() as u64
                + self.num_key_bytes * config.byte_factor_key() as u64)
                * config.byte_cost() as u64
        }
    }

    #[cfg(feature = "rand")]
    #[test]
    fn test_compute_rent_structure() {
        use iota_sdk::types::block::{output::Rent, rand::output};
        use pretty_assertions::assert_eq;

        use super::RentStructureBytes;

        let protocol_params = iota_sdk::types::block::protocol::protocol_parameters();

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
