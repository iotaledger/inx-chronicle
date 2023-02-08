// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the ledger.

use std::ops::{AddAssign, SubAssign};

use derive_more::{AddAssign, SubAssign};

pub(super) use self::{
    active_addresses::{AddressActivityAnalytics, AddressActivityMeasurement},
    address_balance::{AddressBalanceMeasurement, AddressBalancesAnalytics},
    base_token::BaseTokenActivityMeasurement,
    ledger_outputs::LedgerOutputMeasurement,
    ledger_size::{LedgerSizeAnalytics, LedgerSizeMeasurement},
    output_activity::OutputActivityMeasurement,
    unclaimed_tokens::UnclaimedTokenMeasurement,
    unlock_conditions::UnlockConditionMeasurement,
};
use crate::{
    analytics::{Analytics, AnalyticsContext, PerMilestone, TimeInterval},
    types::{
        ledger::{LedgerOutput, LedgerSpent},
        stardust::block::{output::TokenAmount, Output},
    },
};

mod active_addresses;
mod address_balance;
mod base_token;
mod ledger_outputs;
mod ledger_size;
mod output_activity;
mod unclaimed_tokens;
mod unlock_conditions;

#[derive(Copy, Clone, Debug, Default, AddAssign, SubAssign)]
pub(crate) struct CountAndAmount {
    pub(crate) count: usize,
    pub(crate) amount: TokenAmount,
}

impl AddAssign<&LedgerOutput> for CountAndAmount {
    fn add_assign(&mut self, rhs: &LedgerOutput) {
        self.count += 1;
        self.amount += rhs.output.amount();
    }
}

impl SubAssign<&LedgerSpent> for CountAndAmount {
    fn sub_assign(&mut self, rhs: &LedgerSpent) {
        self.count -= 1;
        self.amount -= rhs.output.output.amount();
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use output_activity::OutputActivityMeasurement;

    use super::UnclaimedTokenMeasurement;
    use crate::{
        analytics::{ledger::output_activity, test::TestContext, Analytics},
        types::{
            ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, RentStructureBytes, SpentMetadata},
            stardust::block::{
                output::{AliasId, AliasOutput, BasicOutput, NftId, NftOutput, OutputId, TokenAmount},
                payload::TransactionId,
                Address, BlockId, Output,
            },
        },
    };

    fn rand_output_with_value(amount: TokenAmount) -> Output {
        // We use `BasicOutput`s in the genesis.
        let mut output = BasicOutput::rand(&iota_types::block::protocol::protocol_parameters());
        output.amount = amount;
        Output::Basic(output)
    }

    #[test]
    fn test_claiming() {
        let protocol_params = iota_types::block::protocol::protocol_parameters();

        // All the unclaimed tokens
        let ledger_state = (1u32..=5)
            .map(|i| LedgerOutput {
                output_id: OutputId::rand(),
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
                output: rand_output_with_value((i as u64).into()),
                block_id: BlockId::rand(),
                booked: MilestoneIndexTimestamp {
                    milestone_index: 0.into(),
                    milestone_timestamp: 10000.into(),
                },
            })
            .collect::<Vec<_>>();

        let consumed = ledger_state
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, output)| LedgerSpent {
                output,
                spent_metadata: SpentMetadata {
                    transaction_id: TransactionId::rand(),
                    spent: MilestoneIndexTimestamp {
                        milestone_index: (i as u32 + 1).into(),
                        milestone_timestamp: (i as u32 + 10001).into(),
                    },
                },
            })
            .map(|output| (output.spent_metadata.spent, output))
            .collect::<BTreeMap<_, _>>();

        let transactions = consumed
            .into_iter()
            .map(|(at, output)| {
                (
                    at,
                    (
                        LedgerOutput {
                            output_id: OutputId::rand(),
                            rent_structure: RentStructureBytes {
                                num_key_bytes: 0,
                                num_data_bytes: 100,
                            },
                            output: rand_output_with_value(output.amount()),
                            block_id: BlockId::rand(),
                            booked: MilestoneIndexTimestamp {
                                milestone_index: output.spent_metadata.spent.milestone_index,
                                milestone_timestamp: output.spent_metadata.spent.milestone_timestamp,
                            },
                        },
                        output,
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let mut unclaimed_tokens = UnclaimedTokenMeasurement::init(&ledger_state);
        assert_eq!(unclaimed_tokens.unclaimed_count, 5);
        assert_eq!(unclaimed_tokens.unclaimed_value.0, (1..=5).sum::<u64>());

        for (i, (at, (created, consumed))) in transactions.into_iter().enumerate() {
            let ctx = TestContext {
                at,
                params: protocol_params.clone().into(),
            };
            unclaimed_tokens.begin_milestone(&ctx);
            unclaimed_tokens.handle_transaction(&[consumed], &[created], &ctx);
            let unclaimed_tokens_measurement = unclaimed_tokens.end_milestone(&ctx).unwrap();
            assert_eq!(unclaimed_tokens_measurement.at, ctx.at);
            assert_eq!(unclaimed_tokens_measurement.inner.unclaimed_count, 5 - i - 1);
            assert_eq!(
                unclaimed_tokens_measurement.inner.unclaimed_value.0,
                (1..=5).sum::<u64>() - (1..=(i as u64 + 1)).sum::<u64>()
            )
        }
    }

    #[test]
    fn test_alias_output_activity() {
        let protocol_params = iota_types::block::protocol::protocol_parameters();

        // The id of the spending transaction.
        let transaction_id = TransactionId::rand();

        // Creates a transaction input from an Alias output.
        let tx_input = |output| LedgerOutput {
            output_id: OutputId::rand(),
            rent_structure: RentStructureBytes {
                num_key_bytes: 0,
                num_data_bytes: 100,
            },
            output: Output::Alias(output),
            block_id: BlockId::rand(),
            booked: MilestoneIndexTimestamp {
                milestone_index: 2.into(),
                milestone_timestamp: 12345.into(),
            },
        };

        // Creates a transaction output from an Alias output.
        let tx_output = |(index, output)| LedgerOutput {
            output_id: OutputId {
                transaction_id,
                index: index as u16,
            },
            rent_structure: RentStructureBytes {
                num_key_bytes: 0,
                num_data_bytes: 100,
            },
            output: Output::Alias(output),
            block_id: BlockId::rand(),
            booked: MilestoneIndexTimestamp {
                milestone_index: 2.into(),
                milestone_timestamp: 12345.into(),
            },
        };

        // Spends an Alias output in the given transaction.
        let spend_output = |output| LedgerSpent {
            output,
            spent_metadata: SpentMetadata {
                transaction_id,
                spent: MilestoneIndexTimestamp {
                    milestone_index: 2.into(),
                    milestone_timestamp: 12345.into(),
                },
            },
        };

        let mut created_alias = AliasOutput::rand(&protocol_params);
        created_alias.alias_id = AliasId::implicit();
        let unchanged_alias = AliasOutput::rand(&protocol_params);
        let state_changing_alias = AliasOutput::rand(&protocol_params);
        let mut state_changed_alias = state_changing_alias.clone();
        state_changed_alias.state_index += 1;
        let governor_changing_alias = AliasOutput::rand(&protocol_params);
        let mut governor_changed_alias = governor_changing_alias.clone();
        governor_changed_alias.governor_address_unlock_condition.address = Address::rand_ed25519();
        let destroyed_alias = AliasOutput::rand(&protocol_params);

        // Create and insert transaction outputs.
        let created = vec![
            created_alias,
            unchanged_alias.clone(),
            state_changed_alias,
            governor_changed_alias,
        ]
        .into_iter()
        .enumerate()
        .map(tx_output)
        .collect::<Vec<_>>();

        // Create and insert transaction inputs.
        let consumed = vec![
            unchanged_alias,
            state_changing_alias,
            governor_changing_alias,
            destroyed_alias,
        ]
        .into_iter()
        .map(tx_input)
        .map(spend_output)
        .collect::<Vec<_>>();

        let mut output_activity = OutputActivityMeasurement::default();
        let ctx = TestContext {
            at: MilestoneIndexTimestamp {
                milestone_index: 2.into(),
                milestone_timestamp: 12345.into(),
            },
            params: protocol_params.into(),
        };
        output_activity.begin_milestone(&ctx);
        output_activity.handle_transaction(&consumed, &created, &ctx);
        let output_activity_measurement = output_activity.end_milestone(&ctx).unwrap();

        assert_eq!(output_activity_measurement.at, ctx.at);
        assert_eq!(output_activity_measurement.inner.alias.created_count, 1);
        assert_eq!(output_activity_measurement.inner.alias.governor_changed_count, 1);
        assert_eq!(output_activity_measurement.inner.alias.state_changed_count, 1);
        assert_eq!(output_activity_measurement.inner.alias.destroyed_count, 1);
    }

    #[test]
    fn test_nft_output_activity() {
        let protocol_params = iota_types::block::protocol::protocol_parameters();

        // The id of the spending transaction.
        let transaction_id = TransactionId::rand();

        // Creates a transaction input from an NFT output.
        let tx_input = |output| LedgerOutput {
            output_id: OutputId::rand(),
            rent_structure: RentStructureBytes {
                num_key_bytes: 0,
                num_data_bytes: 100,
            },
            output: Output::Nft(output),
            block_id: BlockId::rand(),
            booked: MilestoneIndexTimestamp {
                milestone_index: 2.into(),
                milestone_timestamp: 12345.into(),
            },
        };

        // Creates a transaction output from an NFT output.
        let tx_output = |(index, output)| LedgerOutput {
            output_id: OutputId {
                transaction_id,
                index: index as u16,
            },
            rent_structure: RentStructureBytes {
                num_key_bytes: 0,
                num_data_bytes: 100,
            },
            output: Output::Nft(output),
            block_id: BlockId::rand(),
            booked: MilestoneIndexTimestamp {
                milestone_index: 2.into(),
                milestone_timestamp: 12345.into(),
            },
        };

        // Spends an NFT output in the given transaction.
        let spend_output = |output| LedgerSpent {
            output,
            spent_metadata: SpentMetadata {
                transaction_id,
                spent: MilestoneIndexTimestamp {
                    milestone_index: 2.into(),
                    milestone_timestamp: 12345.into(),
                },
            },
        };

        let mut created_nft = NftOutput::rand(&protocol_params);
        created_nft.nft_id = NftId::implicit();
        let transferred_nft1 = NftOutput::rand(&protocol_params);
        let transferred_nft2 = NftOutput::rand(&protocol_params);
        let destroyed_nft1 = NftOutput::rand(&protocol_params);
        let destroyed_nft2 = NftOutput::rand(&protocol_params);

        // Create and insert transaction outputs.
        let created = vec![created_nft, transferred_nft1.clone(), transferred_nft2.clone()]
            .into_iter()
            .enumerate()
            .map(tx_output)
            .collect::<Vec<_>>();

        // Create and insert transaction inputs.
        let consumed = vec![transferred_nft1, transferred_nft2, destroyed_nft1, destroyed_nft2]
            .into_iter()
            .map(tx_input)
            .map(spend_output)
            .collect::<Vec<_>>();

        let mut output_activity = OutputActivityMeasurement::default();
        let ctx = TestContext {
            at: MilestoneIndexTimestamp {
                milestone_index: 2.into(),
                milestone_timestamp: 12345.into(),
            },
            params: protocol_params.clone().into(),
        };
        output_activity.begin_milestone(&ctx);
        output_activity.handle_transaction(&consumed, &created, &ctx);
        let output_activity_measurement = output_activity.end_milestone(&ctx).unwrap();

        assert_eq!(output_activity_measurement.at, ctx.at);
        assert_eq!(output_activity_measurement.inner.nft.created_count, 1);
        assert_eq!(output_activity_measurement.inner.nft.transferred_count, 2);
        assert_eq!(output_activity_measurement.inner.nft.destroyed_count, 2);

        let mut created_nft = NftOutput::rand(&protocol_params);
        created_nft.nft_id = NftId::implicit();
        let transferred_nft1 = NftOutput::rand(&protocol_params);
        let transferred_nft2 = NftOutput::rand(&protocol_params);
        let transferred_nft3 = NftOutput::rand(&protocol_params);

        let created = std::iter::once(created_nft)
            .map(|output| LedgerOutput {
                output_id: OutputId::rand(),
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
                output: Output::Nft(output),
                block_id: BlockId::rand(),
                booked: MilestoneIndexTimestamp {
                    milestone_index: 1.into(),
                    milestone_timestamp: 1234.into(),
                },
            })
            .chain(
                vec![
                    transferred_nft1.clone(),
                    transferred_nft2.clone(),
                    transferred_nft3.clone(),
                ]
                .into_iter()
                .enumerate()
                .map(tx_output),
            )
            .collect::<Vec<_>>();

        // Create and insert transaction inputs.
        let consumed = vec![transferred_nft1, transferred_nft2, transferred_nft3]
            .into_iter()
            .map(tx_input)
            .map(spend_output)
            .collect::<Vec<_>>();

        let mut output_activity = OutputActivityMeasurement::default();
        output_activity.begin_milestone(&ctx);
        output_activity.handle_transaction(&consumed, &created, &ctx);
        let output_activity_measurement = output_activity.end_milestone(&ctx).unwrap();

        assert_eq!(output_activity_measurement.at, ctx.at);
        assert_eq!(output_activity_measurement.inner.nft.created_count, 0);
        assert_eq!(output_activity_measurement.inner.nft.transferred_count, 3);
        assert_eq!(output_activity_measurement.inner.nft.destroyed_count, 0);
    }
}
