// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use chronicle::{
        db::collections::{OutputCollection, OutputMetadataResult, OutputWithMetadataResult},
        types::stardust::{
            block::{
                output::{AliasId, AliasOutput, NftId, NftOutput, OutputId},
                payload::TransactionId,
                Address, BlockId, Output,
            },
            ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, RentStructureBytes, SpentMetadata},
        },
    };

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_outputs() {
        let db = setup_database("test-outputs").await.unwrap();
        let output_collection = setup_collection::<OutputCollection>(&db).await.unwrap();

        let protocol_params = iota_types::block::protocol::protocol_parameters();

        let outputs = std::iter::repeat_with(|| Output::rand(&protocol_params))
            .take(100)
            .map(|output| LedgerOutput {
                output_id: OutputId::rand(),
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
                output,
                block_id: BlockId::rand(),
                booked: MilestoneIndexTimestamp {
                    milestone_index: 1.into(),
                    milestone_timestamp: 12345.into(),
                },
            })
            .collect::<Vec<_>>();

        output_collection.insert_unspent_outputs(&outputs).await.unwrap();

        for output in &outputs {
            assert_eq!(
                output_collection
                    .get_spending_transaction_metadata(&output.output_id)
                    .await
                    .unwrap()
                    .as_ref(),
                None,
            );
        }

        for output in &outputs {
            assert_eq!(
                output_collection.get_output(&output.output_id).await.unwrap().as_ref(),
                Some(&output.output),
            );
        }

        for output in &outputs {
            assert_eq!(
                output_collection
                    .get_output_metadata(&output.output_id, 1.into())
                    .await
                    .unwrap(),
                Some(OutputMetadataResult {
                    output_id: output.output_id,
                    block_id: output.block_id,
                    booked: output.booked,
                    spent_metadata: None,
                }),
            );
        }

        for output in &outputs {
            assert_eq!(
                output_collection
                    .get_output_with_metadata(&output.output_id, 1.into())
                    .await
                    .unwrap(),
                Some(OutputWithMetadataResult {
                    output: output.output.clone(),
                    metadata: OutputMetadataResult {
                        output_id: output.output_id,
                        block_id: output.block_id,
                        booked: output.booked,
                        spent_metadata: None,
                    }
                }),
            );
        }

        let outputs = outputs
            .into_iter()
            .map(|output| LedgerSpent {
                output,
                spent_metadata: SpentMetadata {
                    transaction_id: TransactionId::rand(),
                    spent: MilestoneIndexTimestamp {
                        milestone_index: 1.into(),
                        milestone_timestamp: 23456.into(),
                    },
                },
            })
            .collect::<Vec<_>>();

        output_collection.update_spent_outputs(&outputs).await.unwrap();

        for output in &outputs {
            assert_eq!(
                output_collection
                    .get_output(&output.output.output_id)
                    .await
                    .unwrap()
                    .as_ref(),
                Some(&output.output.output),
            );
        }

        for output in &outputs {
            assert_eq!(
                output_collection
                    .get_output_metadata(&output.output.output_id, 1.into())
                    .await
                    .unwrap(),
                Some(OutputMetadataResult {
                    output_id: output.output.output_id,
                    block_id: output.output.block_id,
                    booked: output.output.booked,
                    spent_metadata: Some(output.spent_metadata),
                }),
            );
        }

        for output in &outputs {
            assert_eq!(
                output_collection
                    .get_output_with_metadata(&output.output.output_id, 1.into())
                    .await
                    .unwrap(),
                Some(OutputWithMetadataResult {
                    output: output.output.output.clone(),
                    metadata: OutputMetadataResult {
                        output_id: output.output.output_id,
                        block_id: output.output.block_id,
                        booked: output.output.booked,
                        spent_metadata: Some(output.spent_metadata),
                    }
                }),
            );
        }

        for output in &outputs {
            assert_eq!(
                output_collection
                    .get_spending_transaction_metadata(&output.output.output_id)
                    .await
                    .unwrap()
                    .as_ref(),
                Some(&output.spent_metadata),
            );
        }

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_alias_outputs_1() {
        let db = setup_database("test-alias-outputs").await.unwrap();
        let output_collection = setup_collection::<OutputCollection>(&db).await.unwrap();

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
        let tx_outputs = vec![
            created_alias,
            unchanged_alias.clone(),
            state_changed_alias,
            governor_changed_alias,
        ]
        .into_iter()
        .enumerate()
        .map(tx_output)
        .collect::<Vec<_>>();
        output_collection.insert_unspent_outputs(&tx_outputs).await.unwrap();

        // Create and insert transaction inputs.
        let tx_inputs = vec![
            unchanged_alias,
            state_changing_alias,
            governor_changing_alias,
            destroyed_alias,
        ]
        .into_iter()
        .map(tx_input)
        .collect::<Vec<_>>();
        output_collection.insert_unspent_outputs(&tx_inputs).await.unwrap();

        let spent_tx_inputs = tx_inputs.into_iter().map(spend_output).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&spent_tx_inputs).await.unwrap();

        let analytics = output_collection
            .get_alias_output_activity_analytics(2.into())
            .await
            .unwrap();

        assert_eq!(analytics.created_count, 1);
        assert_eq!(analytics.governor_changed_count, 1);
        assert_eq!(analytics.state_changed_count, 1);
        assert_eq!(analytics.destroyed_count, 1);

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_nft_outputs_1() {
        let db = setup_database("test-nft-outputs-1").await.unwrap();
        let output_collection = setup_collection::<OutputCollection>(&db).await.unwrap();

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
        let tx_outputs = vec![created_nft, transferred_nft1.clone(), transferred_nft2.clone()]
            .into_iter()
            .enumerate()
            .map(tx_output)
            .collect::<Vec<_>>();
        output_collection.insert_unspent_outputs(&tx_outputs).await.unwrap();

        // Create and insert transaction inputs.
        let tx_inputs = vec![transferred_nft1, transferred_nft2, destroyed_nft1, destroyed_nft2]
            .into_iter()
            .map(tx_input)
            .collect::<Vec<_>>();
        output_collection.insert_unspent_outputs(&tx_inputs).await.unwrap();

        let spent_tx_inputs = tx_inputs.into_iter().map(spend_output).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&spent_tx_inputs).await.unwrap();

        let analytics = output_collection
            .get_nft_output_activity_analytics(2.into())
            .await
            .unwrap();

        assert_eq!(analytics.created_count, 1);
        assert_eq!(analytics.transferred_count, 2);
        assert_eq!(analytics.destroyed_count, 2);

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_nft_outputs_2() {
        let db = setup_database("test-nft-outputs-2").await.unwrap();
        let output_collection = setup_collection::<OutputCollection>(&db).await.unwrap();

        let protocol_params = iota_types::block::protocol::protocol_parameters();

        // Create the inputs and outputs of a transaction in the form of ledger updates.
        let transaction_id = TransactionId::rand();

        // Makes transaction inputs
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

        // Makes transaction outputs
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

        // Makes spent metadata.
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
        let transferred_nft3 = NftOutput::rand(&protocol_params);

        let tx_outputs = std::iter::once(created_nft)
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
        output_collection.insert_unspent_outputs(&tx_outputs).await.unwrap();

        // Create and insert transaction inputs.
        let tx_inputs = vec![transferred_nft1, transferred_nft2, transferred_nft3]
            .into_iter()
            .map(tx_input)
            .collect::<Vec<_>>();
        output_collection.insert_unspent_outputs(&tx_inputs).await.unwrap();

        let spent_tx_inputs = tx_inputs.into_iter().map(spend_output).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&spent_tx_inputs).await.unwrap();

        let analytics = output_collection
            .get_nft_output_activity_analytics(2.into())
            .await
            .unwrap();

        assert_eq!(analytics.created_count, 0);
        assert_eq!(analytics.transferred_count, 3);
        assert_eq!(analytics.destroyed_count, 0);

        teardown(db).await;
    }
}
