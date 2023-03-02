// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the ledger.

use serde::{Deserialize, Serialize};

pub(super) use self::{
    active_addresses::{AddressActivityAnalytics, AddressActivityMeasurement},
    address_balance::{AddressBalanceMeasurement, AddressBalancesAnalytics},
    base_token::BaseTokenActivityMeasurement,
    ledger_outputs::LedgerOutputMeasurement,
    ledger_size::{LedgerSizeAnalytics, LedgerSizeMeasurement},
    output_activity::OutputActivityMeasurement,
    transaction_size::TransactionSizeMeasurement,
    unclaimed_tokens::UnclaimedTokenMeasurement,
    unlock_conditions::UnlockConditionMeasurement,
};
use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::{LedgerOutput, LedgerSpent, Output, TokenAmount},
};

mod active_addresses;
mod address_balance;
mod base_token;
mod ledger_outputs;
mod ledger_size;
mod output_activity;
mod transaction_size;
mod unclaimed_tokens;
mod unlock_conditions;

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct CountAndAmount {
    pub(crate) count: usize,
    pub(crate) amount: TokenAmount,
}

impl CountAndAmount {
    fn wrapping_add(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_add(rhs.count),
            amount: TokenAmount(self.amount.0.wrapping_add(rhs.amount.0)),
        }
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_sub(rhs.count),
            amount: TokenAmount(self.amount.0.wrapping_sub(rhs.amount.0)),
        }
    }

    fn add_output(&mut self, rhs: &LedgerOutput) {
        self.count += 1;
        self.amount += rhs.amount();
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        analytics::{test::TestContext, Analytics},
        model::{
            Address, AliasId, AliasOutput, BasicOutput, BlockId, LedgerOutput, LedgerSpent, MilestoneIndexTimestamp,
            NftId, NftOutput, Output, OutputId, RentStructureBytes, SpentMetadata, TokenAmount, TransactionId,
        },
    };

    fn rand_output_with_amount(amount: TokenAmount) -> Output {
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
                output: rand_output_with_amount((i as u64).into()),
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
                            output: rand_output_with_amount(output.amount()),
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
        assert_eq!(unclaimed_tokens.unclaimed_amount.0, (1..=5).sum::<u64>());

        for (i, (at, (created, consumed))) in transactions.into_iter().enumerate() {
            let ctx = TestContext {
                at,
                params: protocol_params.clone().into(),
            };

            unclaimed_tokens.handle_transaction(&[consumed], &[created], &ctx);
            let unclaimed_tokens_measurement = unclaimed_tokens.take_measurement(&ctx);
            assert_eq!(unclaimed_tokens_measurement.unclaimed_count, 5 - i - 1);
            assert_eq!(
                unclaimed_tokens_measurement.unclaimed_amount.0,
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

        output_activity.handle_transaction(&consumed, &created, &ctx);
        let output_activity_measurement = output_activity.take_measurement(&ctx);

        assert_eq!(output_activity_measurement.alias.created_count, 1);
        assert_eq!(output_activity_measurement.alias.governor_changed_count, 1);
        assert_eq!(output_activity_measurement.alias.state_changed_count, 1);
        assert_eq!(output_activity_measurement.alias.destroyed_count, 1);
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

        output_activity.handle_transaction(&consumed, &created, &ctx);
        let output_activity_measurement = output_activity.take_measurement(&ctx);

        assert_eq!(output_activity_measurement.nft.created_count, 1);
        assert_eq!(output_activity_measurement.nft.transferred_count, 2);
        assert_eq!(output_activity_measurement.nft.destroyed_count, 2);

        let mut created_nft = NftOutput::rand(&protocol_params);
        created_nft.nft_id = NftId::implicit();
        let transferred_nft1 = NftOutput::rand(&protocol_params);
        let transferred_nft2 = NftOutput::rand(&protocol_params);
        let transferred_nft3 = NftOutput::rand(&protocol_params);

        // Created on milestone 1
        let created = [LedgerOutput {
            output_id: OutputId::rand(),
            rent_structure: RentStructureBytes {
                num_key_bytes: 0,
                num_data_bytes: 100,
            },
            output: Output::Nft(created_nft),
            block_id: BlockId::rand(),
            booked: MilestoneIndexTimestamp {
                milestone_index: 1.into(),
                milestone_timestamp: 1234.into(),
            },
        }];

        let ctx = TestContext {
            at: MilestoneIndexTimestamp {
                milestone_index: 1.into(),
                milestone_timestamp: 1234.into(),
            },
            params: protocol_params.clone().into(),
        };
        let mut output_activity = OutputActivityMeasurement::default();

        output_activity.handle_transaction(&[], &created, &ctx);
        let output_activity_measurement = output_activity.take_measurement(&ctx);

        assert_eq!(output_activity_measurement.nft.created_count, 1);
        assert_eq!(output_activity_measurement.nft.transferred_count, 0);
        assert_eq!(output_activity_measurement.nft.destroyed_count, 0);

        // Created on milestone 2
        let created = [
            transferred_nft1.clone(),
            transferred_nft2.clone(),
            transferred_nft3.clone(),
        ]
        .into_iter()
        .enumerate()
        .map(tx_output)
        .collect::<Vec<_>>();

        // Consumed on milestone 2
        let consumed = vec![transferred_nft1, transferred_nft2, transferred_nft3]
            .into_iter()
            .map(tx_input)
            .map(spend_output)
            .collect::<Vec<_>>();

        let ctx = TestContext {
            at: MilestoneIndexTimestamp {
                milestone_index: 2.into(),
                milestone_timestamp: 12345.into(),
            },
            params: protocol_params.into(),
        };
        let mut output_activity = OutputActivityMeasurement::default();

        output_activity.handle_transaction(&consumed, &created, &ctx);
        let output_activity_measurement = output_activity.take_measurement(&ctx);

        assert_eq!(output_activity_measurement.nft.created_count, 0);
        assert_eq!(output_activity_measurement.nft.transferred_count, 3);
        assert_eq!(output_activity_measurement.nft.destroyed_count, 0);
    }

    fn rand_output_with_address_and_amount(
        address: Address,
        amount: u64,
        ctx: &iota_types::block::protocol::ProtocolParameters,
    ) -> Output {
        use iota_types::block::{
            address::Address,
            output::{unlock_condition::AddressUnlockCondition, BasicOutput},
            rand::output::feature::rand_allowed_features,
        };
        let output = BasicOutput::build_with_amount(amount)
            .unwrap()
            .with_features(rand_allowed_features(BasicOutput::ALLOWED_FEATURES))
            .add_unlock_condition(AddressUnlockCondition::from(Address::from(address)).into())
            .finish(ctx.token_supply())
            .unwrap();
        Output::Basic(output.into())
    }

    #[test]
    fn test_base_tokens() {
        let protocol_params = iota_types::block::protocol::protocol_parameters();

        let address_1 = Address::rand_ed25519();
        let address_2 = Address::rand_ed25519();
        let address_3 = Address::rand_ed25519();

        let transaction_id = TransactionId::rand();

        let milestone = MilestoneIndexTimestamp {
            milestone_index: 1.into(),
            milestone_timestamp: 10000.into(),
        };

        let spend_output = |output| LedgerSpent {
            output,
            spent_metadata: SpentMetadata {
                transaction_id,
                spent: milestone,
            },
        };

        let from_address = |address, amount| {
            spend_output(LedgerOutput {
                output_id: OutputId::rand(),
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
                output: rand_output_with_address_and_amount(address, amount, &protocol_params),
                block_id: BlockId::rand(),
                booked: milestone,
            })
        };

        let to_address = |address, amount| LedgerOutput {
            output_id: OutputId::rand(),
            rent_structure: RentStructureBytes {
                num_key_bytes: 0,
                num_data_bytes: 100,
            },
            output: rand_output_with_address_and_amount(address, amount, &protocol_params),
            block_id: BlockId::rand(),
            booked: milestone,
        };

        let consumed = [
            from_address(address_1, 50),
            from_address(address_1, 20),
            from_address(address_1, 35),
            from_address(address_2, 5),
            from_address(address_2, 15),
            from_address(address_3, 25),
            from_address(address_3, 55),
            from_address(address_3, 75),
            from_address(address_3, 80),
            from_address(address_3, 100),
        ];

        let created = [
            to_address(address_1, 60),
            to_address(address_1, 20),
            to_address(address_1, 200),
            to_address(address_2, 40),
            to_address(address_2, 50),
            to_address(address_3, 45),
            to_address(address_3, 45),
        ];

        let ctx = TestContext {
            at: milestone,
            params: protocol_params.clone().into(),
        };
        let mut base_tokens = BaseTokenActivityMeasurement::default();

        base_tokens.handle_transaction(&consumed, &created, &ctx);
        let base_tokens_measurement = base_tokens.take_measurement(&ctx);

        assert_eq!(base_tokens_measurement.booked_amount.0, 460);
        // Address 1 has delta +175, Address 2 has delta +70, Address 3 has delta -255
        assert_eq!(base_tokens_measurement.transferred_amount.0, 245)
    }
}
