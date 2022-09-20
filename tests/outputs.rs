// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use std::collections::HashMap;

    use chronicle::{
        db::collections::{
            LedgerUpdateByAddressRecord, LedgerUpdateByMilestoneRecord, LedgerUpdateCollection, OutputCollection,
            OutputMetadataResult, OutputWithMetadataResult, SortOrder,
        },
        types::{
            ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, SpentMetadata},
            stardust::block::{output::OutputId, payload::TransactionId, Address, BlockId, Output},
        },
    };
    use futures::stream::TryStreamExt;

    use super::common::connect_to_test_db;

    #[tokio::test]
    async fn test_outputs() {
        let db = connect_to_test_db("test-outputs").await.unwrap();
        db.clear().await.unwrap();
        let collection = db.collection::<OutputCollection>();
        collection.create_indexes().await.unwrap();

        let outputs = std::iter::repeat_with(Output::rand)
            .take(100)
            .map(|output| LedgerOutput {
                output_id: OutputId::rand(),
                output,
                block_id: BlockId::rand(),
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
                    block_id: output.block_id,
                    booked: output.booked,
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
                    block_id: output.output.block_id,
                    booked: output.output.booked,
                    spent_metadata: Some(output.spent_metadata),
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

    #[tokio::test]
    async fn test_ledger_updates() {
        let db = connect_to_test_db("test-ledger-updates").await.unwrap();
        db.clear().await.unwrap();
        let collection = db.collection::<OutputCollection>();
        collection.create_indexes().await.unwrap();

        let address = Address::rand_ed25519();

        let outputs = std::iter::repeat_with(Output::rand)
            .take(100)
            .map(|mut output| {
                if let Output::Basic(o) = &mut output {
                    o.address_unlock_condition.address = address;
                }
                LedgerOutput {
                    output_id: OutputId::rand(),
                    output,
                    block_id: BlockId::rand(),
                    booked: MilestoneIndexTimestamp {
                        milestone_index: 1.into(),
                        milestone_timestamp: 12345.into(),
                    },
                }
            })
            .collect::<Vec<_>>();

        collection.insert_unspent_outputs(&outputs).await.unwrap();

        collection.create_ledger_updates().await.unwrap();

        let outputs_map = outputs
            .iter()
            .filter_map(|o| {
                if o.output.owning_address().is_some() {
                    Some((
                        o.output_id,
                        LedgerUpdateByMilestoneRecord {
                            address: *o.output.owning_address().unwrap(),
                            output_id: o.output_id,
                            is_spent: false,
                        },
                    ))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let ledger_updates = db
            .collection::<LedgerUpdateCollection>()
            .get_ledger_updates_by_milestone(1.into(), 100, None)
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert!(ledger_updates.len() == outputs_map.len());
        for update in &ledger_updates {
            assert_eq!(outputs_map.get(&update.output_id).unwrap(), update);
        }

        // Spend some of the previous outputs
        let spent_outputs = outputs
            .iter()
            .enumerate()
            .filter_map(|(i, output)| {
                if i % 2 == 0 {
                    Some(LedgerSpent {
                        output: output.clone(),
                        spent_metadata: SpentMetadata {
                            transaction_id: TransactionId::rand(),
                            spent: MilestoneIndexTimestamp {
                                milestone_index: 2.into(),
                                milestone_timestamp: 23456.into(),
                            },
                        },
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        collection.update_spent_outputs(&spent_outputs).await.unwrap();

        // Create some new unspent outputs
        let unspent_outputs = std::iter::repeat_with(Output::rand)
            .take(100)
            .map(|mut output| {
                if let Output::Basic(o) = &mut output {
                    o.address_unlock_condition.address = address;
                }
                LedgerOutput {
                    output_id: OutputId::rand(),
                    output,
                    block_id: BlockId::rand(),
                    booked: MilestoneIndexTimestamp {
                        milestone_index: 2.into(),
                        milestone_timestamp: 23456.into(),
                    },
                }
            })
            .collect::<Vec<_>>();

        collection.insert_unspent_outputs(&unspent_outputs).await.unwrap();

        collection.merge_into_ledger_updates(2.into()).await.unwrap();

        let outputs_map = spent_outputs
            .iter()
            .map(|o| (&o.output, true))
            .chain(unspent_outputs.iter().map(|o| (o, false)))
            .filter_map(|(o, is_spent)| {
                if o.output.owning_address().is_some() {
                    Some((
                        o.output_id,
                        LedgerUpdateByMilestoneRecord {
                            address: *o.output.owning_address().unwrap(),
                            output_id: o.output_id,
                            is_spent,
                        },
                    ))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let ledger_updates = db
            .collection::<LedgerUpdateCollection>()
            .get_ledger_updates_by_milestone(2.into(), 200, None)
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert!(ledger_updates.len() == outputs_map.len());
        for update in &ledger_updates {
            assert_eq!(outputs_map.get(&update.output_id).unwrap(), update);
        }

        let outputs_map = spent_outputs
            .iter()
            .map(|o| (&o.output, Some(o.spent_metadata)))
            .chain(unspent_outputs.iter().map(|o| (o, None)))
            .chain(outputs.iter().map(|o| (o, None)))
            .filter_map(|(o, spent_metadata)| {
                if o.output.owning_address() == Some(&address) {
                    Some((
                        (o.output_id, spent_metadata.is_some()),
                        LedgerUpdateByAddressRecord {
                            at: if let Some(spent_metadata) = spent_metadata {
                                spent_metadata.spent
                            } else {
                                o.booked
                            },
                            output_id: o.output_id,
                            is_spent: spent_metadata.is_some(),
                        },
                    ))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let ledger_updates = db
            .collection::<LedgerUpdateCollection>()
            .get_ledger_updates_by_address(&address, 200, None, SortOrder::Newest)
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert!(ledger_updates.len() == outputs_map.len());
        for update in &ledger_updates {
            assert_eq!(outputs_map.get(&(update.output_id, update.is_spent)).unwrap(), update);
        }
    }
}
