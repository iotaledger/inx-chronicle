// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

use bee_block_stardust as bee;
use chronicle::{
    db::collections::{OutputCollection, OutputMetadataResult, OutputWithMetadataResult},
    types::{
        ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, SpentMetadata},
        stardust::{
            block::{
                output::OutputId,
                payload::{TransactionEssence, TransactionPayload},
                BlockId,
            },
            util::get_test_transaction_block,
        },
    },
};
use common::connect_to_test_db;

#[tokio::test]
async fn test_outputs() {
    let db = connect_to_test_db("test-outputs").await.unwrap();
    db.clear().await.unwrap();
    let collection = db.collection::<OutputCollection>();
    collection.create_indexes().await.unwrap();

    let block = get_test_transaction_block();
    let block_id = BlockId::from(bee::Block::try_from(block.clone()).unwrap().id());
    let transaction_payload = TransactionPayload::try_from(block.payload.unwrap()).unwrap();
    let transaction_id = transaction_payload.transaction_id;
    let TransactionEssence::Regular { outputs, .. } = transaction_payload.essence;
    let outputs = outputs
        .into_vec()
        .into_iter()
        .enumerate()
        .map(|(i, output)| LedgerOutput {
            output_id: OutputId {
                transaction_id,
                index: i as u16,
            },
            output,
            block_id,
            booked: MilestoneIndexTimestamp {
                milestone_index: 1.into(),
                milestone_timestamp: 12345.into(),
            },
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
                block_id,
                booked: output.booked,
                spent_metadata: None,
                ledger_index: 1.into()
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
                    block_id: output.block_id,
                    booked: output.booked,
                    spent_metadata: None,
                    ledger_index: 1.into()
                }
            }),
        );
    }

    let outputs = outputs
        .into_iter()
        .map(|output| LedgerSpent {
            output,
            spent_metadata: SpentMetadata {
                transaction_id: bee_block_stardust::rand::transaction::rand_transaction_id().into(),
                spent: MilestoneIndexTimestamp {
                    milestone_index: 1.into(),
                    milestone_timestamp: 23456.into(),
                },
            },
        })
        .collect::<Vec<_>>();

    collection.update_spent_outputs(&outputs).await.unwrap();

    for output in &outputs {
        assert_eq!(
            collection.get_output(&output.output.output_id).await.unwrap().as_ref(),
            Some(&output.output.output),
        );
    }

    for output in &outputs {
        assert_eq!(
            collection
                .get_output_metadata(&output.output.output_id, 1.into())
                .await
                .unwrap(),
            Some(OutputMetadataResult {
                output_id: output.output.output_id,
                block_id,
                booked: output.output.booked,
                spent_metadata: Some(output.spent_metadata),
                ledger_index: 1.into()
            }),
        );
    }

    for output in &outputs {
        assert_eq!(
            collection
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
                    ledger_index: 1.into()
                }
            }),
        );
    }

    for output in &outputs {
        assert_eq!(
            collection
                .get_spending_transaction_metadata(&output.output.output_id)
                .await
                .unwrap()
                .as_ref(),
            Some(&output.spent_metadata),
        );
    }

    db.drop().await.unwrap();
}
