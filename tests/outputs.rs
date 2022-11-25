// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use chronicle::{
        db::collections::{OutputCollection, OutputMetadataResult, OutputWithMetadataResult},
        types::{
            ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, RentStructureBytes, SpentMetadata},
            stardust::block::{
                output::{AliasId, AliasOutput, NftId, NftOutput, OutputId},
                payload::TransactionId,
                Address, BlockId, Output,
            },
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
    async fn test_alias_outputs() {
        let db = setup_database("test-alias-outputs").await.unwrap();
        let output_collection = setup_collection::<OutputCollection>(&db).await.unwrap();

        let protocol_params = iota_types::block::protocol::protocol_parameters();

        let state_change = |output: &mut AliasOutput| {
            output.state_index += 1;
            output.clone()
        };
        let governor_change = |output: &mut AliasOutput| {
            output.governor_address_unlock_condition.address = Address::rand_ed25519();
            output.clone()
        };
        let ledger_output = |output| LedgerOutput {
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
        let ledger_spent = |output| LedgerSpent {
            output,
            spent_metadata: SpentMetadata {
                transaction_id: TransactionId::rand(),
                spent: MilestoneIndexTimestamp {
                    milestone_index: 2.into(),
                    milestone_timestamp: 12345.into(),
                },
            },
        };

        // c -> t -> s -> s
        let mut output = AliasOutput::rand(&protocol_params);
        let mut created_output = output.clone();
        created_output.alias_id = AliasId::implicit();
        let created_outputs = vec![
            created_output,
            governor_change(&mut output),
            state_change(&mut output),
            state_change(&mut output),
        ]
        .into_iter()
        .map(ledger_output)
        .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs
            .into_iter()
            .take(3)
            .map(ledger_spent)
            .collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.alias.created_count, 1);
        assert_eq!(analytics.alias.governor_changed_count, 1);
        assert_eq!(analytics.alias.state_changed_count, 2);
        assert_eq!(analytics.alias.destroyed_count, 0);

        // t -> s -> s
        let mut output = AliasOutput::rand(&protocol_params);
        let mut created_output = output.clone();
        created_output.alias_id = AliasId::implicit();
        let created_outputs = std::iter::once(created_output)
            .map(|output| LedgerOutput {
                output_id: OutputId::rand(),
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
                output: Output::Alias(output),
                block_id: BlockId::rand(),
                booked: MilestoneIndexTimestamp {
                    milestone_index: 1.into(),
                    milestone_timestamp: 1234.into(),
                },
            })
            .chain(
                vec![
                    governor_change(&mut output),
                    state_change(&mut output),
                    state_change(&mut output),
                ]
                .into_iter()
                .map(ledger_output),
            )
            .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs
            .into_iter()
            .take(3)
            .map(ledger_spent)
            .collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.alias.created_count, 1);
        assert_eq!(analytics.alias.governor_changed_count, 2);
        assert_eq!(analytics.alias.state_changed_count, 4);
        assert_eq!(analytics.alias.destroyed_count, 0);

        // s -> t -> d
        let mut output = AliasOutput::rand(&protocol_params);
        output.state_index += 1;
        let created_outputs = std::iter::once(output.clone())
            .map(|output| LedgerOutput {
                output_id: OutputId::rand(),
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
                output: Output::Alias(output),
                block_id: BlockId::rand(),
                booked: MilestoneIndexTimestamp {
                    milestone_index: 1.into(),
                    milestone_timestamp: 1234.into(),
                },
            })
            .chain(
                vec![state_change(&mut output), governor_change(&mut output)]
                    .into_iter()
                    .map(ledger_output),
            )
            .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs.into_iter().map(ledger_spent).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.alias.created_count, 1);
        assert_eq!(analytics.alias.governor_changed_count, 3);
        assert_eq!(analytics.alias.state_changed_count, 5);
        assert_eq!(analytics.alias.destroyed_count, 1);

        // c -> s -> s -> d
        let mut output = AliasOutput::rand(&protocol_params);
        let mut created_output = output.clone();
        created_output.alias_id = AliasId::implicit();
        let created_outputs = vec![created_output, state_change(&mut output), state_change(&mut output)]
            .into_iter()
            .map(ledger_output)
            .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs.into_iter().map(ledger_spent).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.alias.created_count, 2);
        assert_eq!(analytics.alias.governor_changed_count, 3);
        assert_eq!(analytics.alias.state_changed_count, 7);
        assert_eq!(analytics.alias.destroyed_count, 2);

        // c -> t -> t -> d
        let mut output = AliasOutput::rand(&protocol_params);
        let mut created_output = output.clone();
        created_output.alias_id = AliasId::implicit();
        let created_outputs = vec![
            created_output,
            governor_change(&mut output),
            governor_change(&mut output),
        ]
        .into_iter()
        .map(ledger_output)
        .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs.into_iter().map(ledger_spent).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.alias.created_count, 3);
        assert_eq!(analytics.alias.governor_changed_count, 5);
        assert_eq!(analytics.alias.state_changed_count, 7);
        assert_eq!(analytics.alias.destroyed_count, 3);

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_nft_outputs() {
        let db = setup_database("test-nft-outputs").await.unwrap();
        let output_collection = setup_collection::<OutputCollection>(&db).await.unwrap();

        let protocol_params = iota_types::block::protocol::protocol_parameters();

        let ledger_output = |output| LedgerOutput {
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
        let ledger_spent = |output| LedgerSpent {
            output,
            spent_metadata: SpentMetadata {
                transaction_id: TransactionId::rand(),
                spent: MilestoneIndexTimestamp {
                    milestone_index: 2.into(),
                    milestone_timestamp: 12345.into(),
                },
            },
        };

        // c -> t -> t -> t
        let output = NftOutput::rand(&protocol_params);
        let mut created_output = output.clone();
        created_output.nft_id = NftId::implicit();
        let created_outputs = vec![created_output, output.clone(), output.clone(), output.clone()]
            .into_iter()
            .map(ledger_output)
            .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs
            .into_iter()
            .take(3)
            .map(ledger_spent)
            .collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.nft.created_count, 1);
        assert_eq!(analytics.nft.transferred_count, 3);
        assert_eq!(analytics.nft.destroyed_count, 0);

        // t -> t -> t
        let output = NftOutput::rand(&protocol_params);
        let mut created_output = output.clone();
        created_output.nft_id = NftId::implicit();
        let created_outputs = std::iter::once(created_output)
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
                vec![output.clone(), output.clone(), output.clone()]
                    .into_iter()
                    .map(ledger_output),
            )
            .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs
            .into_iter()
            .take(3)
            .map(ledger_spent)
            .collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.nft.created_count, 1);
        assert_eq!(analytics.nft.transferred_count, 6);
        assert_eq!(analytics.nft.destroyed_count, 0);

        // t -> t -> d
        let output = NftOutput::rand(&protocol_params);
        let created_outputs = std::iter::once(output.clone())
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
            .chain(vec![output.clone(), output.clone()].into_iter().map(ledger_output))
            .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs.into_iter().map(ledger_spent).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.nft.created_count, 1);
        assert_eq!(analytics.nft.transferred_count, 8);
        assert_eq!(analytics.nft.destroyed_count, 1);

        // c -> t -> t -> d
        let output = NftOutput::rand(&protocol_params);
        let mut created_output = output.clone();
        created_output.nft_id = NftId::implicit();
        let created_outputs = vec![created_output, output.clone(), output.clone()]
            .into_iter()
            .map(ledger_output)
            .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs.into_iter().map(ledger_spent).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.nft.created_count, 2);
        assert_eq!(analytics.nft.transferred_count, 10);
        assert_eq!(analytics.nft.destroyed_count, 2);

        // c -> t -> t -> d
        let output = NftOutput::rand(&protocol_params);
        let mut created_output = output.clone();
        created_output.nft_id = NftId::implicit();
        let created_outputs = vec![created_output, output.clone(), output.clone()]
            .into_iter()
            .map(ledger_output)
            .collect::<Vec<_>>();
        output_collection
            .insert_unspent_outputs(&created_outputs)
            .await
            .unwrap();

        let consumed_outputs = created_outputs.into_iter().map(ledger_spent).collect::<Vec<_>>();
        output_collection.update_spent_outputs(&consumed_outputs).await.unwrap();

        let analytics = output_collection.get_output_activity_analytics(2.into()).await.unwrap();

        assert_eq!(analytics.nft.created_count, 3);
        assert_eq!(analytics.nft.transferred_count, 12);
        assert_eq!(analytics.nft.destroyed_count, 3);

        teardown(db).await;
    }
}
