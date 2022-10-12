// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {

    use chronicle::{
        db::{
            collections::{OutputCollection, OutputDocument, OutputMetadataResult, OutputWithMetadataResult},
            MongoDbCollection,
        },
        types::{
            ledger::{LedgerOutput, MilestoneIndexTimestamp, RentStructureBytes, SpentMetadata},
            stardust::block::{output::OutputId, payload::TransactionId, BlockId, Output},
        },
    };

    use super::common::connect_to_test_db;

    #[tokio::test]
    async fn test_outputs() {
        let db = connect_to_test_db("test-outputs").await.unwrap();
        db.clear().await.unwrap();
        let collection = db.collection::<OutputCollection>();
        collection.create_indexes().await.unwrap();

        let protocol_params = bee_block_stardust::protocol::protocol_parameters();

        let outputs = std::iter::repeat_with(|| Output::rand(&protocol_params))
            .take(100)
            .map(|output| {
                OutputDocument::from(LedgerOutput {
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
            })
            .collect::<Vec<_>>();

        collection.insert_unspent_outputs(&outputs).await.unwrap();

        for output in &outputs {
            assert_eq!(
                collection
                    .get_spending_transaction_metadata(&output.output_id)
                    .await
                    .unwrap()
                    .as_ref(),
                None,
            );
        }

        for output in &outputs {
            assert_eq!(
                collection.get_output(&output.output_id).await.unwrap().as_ref(),
                Some(&output.output),
            );
        }

        for output in &outputs {
            assert_eq!(
                collection
                    .get_output_metadata(&output.output_id, 1.into())
                    .await
                    .unwrap(),
                Some(OutputMetadataResult {
                    output_id: output.output_id,
                    block_id: output.metadata.block_id,
                    booked: output.metadata.booked,
                    spent_metadata: None,
                }),
            );
        }

        for output in &outputs {
            assert_eq!(
                collection
                    .get_output_with_metadata(&output.output_id, 1.into())
                    .await
                    .unwrap(),
                Some(OutputWithMetadataResult {
                    output: output.output.clone(),
                    metadata: OutputMetadataResult {
                        output_id: output.output_id,
                        block_id: output.metadata.block_id,
                        booked: output.metadata.booked,
                        spent_metadata: None,
                    }
                }),
            );
        }

        let outputs = outputs
            .into_iter()
            .map(|mut output| {
                output.metadata.spent_metadata.replace(SpentMetadata {
                    transaction_id: TransactionId::rand(),
                    spent: MilestoneIndexTimestamp {
                        milestone_index: 1.into(),
                        milestone_timestamp: 23456.into(),
                    },
                });
                output
            })
            .collect::<Vec<_>>();

        collection.update_spent_outputs(&outputs).await.unwrap();

        for output in &outputs {
            assert_eq!(
                collection.get_output(&output.output_id).await.unwrap().as_ref(),
                Some(&output.output),
            );
        }

        for output in &outputs {
            assert_eq!(
                collection
                    .get_output_metadata(&output.output_id, 1.into())
                    .await
                    .unwrap(),
                Some(OutputMetadataResult {
                    output_id: output.output_id,
                    block_id: output.metadata.block_id,
                    booked: output.metadata.booked,
                    spent_metadata: output.metadata.spent_metadata,
                }),
            );
        }

        for output in &outputs {
            assert_eq!(
                collection
                    .get_output_with_metadata(&output.output_id, 1.into())
                    .await
                    .unwrap(),
                Some(OutputWithMetadataResult {
                    output: output.output.clone(),
                    metadata: OutputMetadataResult {
                        output_id: output.output_id,
                        block_id: output.metadata.block_id,
                        booked: output.metadata.booked,
                        spent_metadata: output.metadata.spent_metadata,
                    }
                }),
            );
        }

        for output in &outputs {
            assert_eq!(
                collection
                    .get_spending_transaction_metadata(&output.output_id)
                    .await
                    .unwrap()
                    .as_ref(),
                output.metadata.spent_metadata.as_ref(),
            );
        }

        db.drop().await.unwrap();
    }
}
