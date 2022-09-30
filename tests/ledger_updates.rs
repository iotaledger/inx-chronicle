// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use std::collections::{HashMap, HashSet};

    use bee_block_stardust::rand::number::rand_number_range;
    use chronicle::{
        db::{
            collections::{
                LedgerUpdateByAddressRecord, LedgerUpdateByMilestoneRecord, LedgerUpdateCollection, SortOrder,
            },
            MongoDb, MongoDbCollectionExt,
        },
        types::{
            ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, RentStructureBytes, SpentMetadata},
            stardust::block::{
                output::{AddressUnlockCondition, BasicOutput, OutputId},
                BlockId, Output,
            },
        },
    };
    use futures::TryStreamExt;

    use super::common::connect_to_test_db;

    #[tokio::test]
    async fn test_ledger_updates_by_address() {
        let (db, collection) = setup("test-ledger-updates-by-address").await;

        let ctx = bee_block_stardust::protocol::protocol_parameters();

        let mut outputs = HashSet::new();
        let address_unlock_condition = AddressUnlockCondition::rand();
        let address = address_unlock_condition.clone().address;

        let ledger_outputs = std::iter::repeat_with(|| (BlockId::rand(), rand_number_range(1..1000), OutputId::rand()))
            .take(50)
            .inspect(|(_, _, output_id)| {
                outputs.insert(output_id.clone());
            })
            .map(|(block_id, amount, output_id)| {
                let output = BasicOutput {
                    amount: amount.into(),
                    native_tokens: Vec::new().into_boxed_slice(),
                    address_unlock_condition: address_unlock_condition.clone(),
                    storage_deposit_return_unlock_condition: None,
                    timelock_unlock_condition: None,
                    expiration_unlock_condition: None,
                    features: Vec::new().into_boxed_slice(),
                };
                LedgerOutput {
                    block_id,
                    booked: MilestoneIndexTimestamp {
                        milestone_index: 0.into(),
                        milestone_timestamp: 12345.into(),
                    },
                    output: Output::Basic(output),
                    output_id,
                    rent_structure: RentStructureBytes {
                        num_key_bytes: 0,
                        num_data_bytes: 100,
                    },
                }
            })
            .chain(
                std::iter::repeat_with(|| (BlockId::rand(), Output::rand(&ctx), OutputId::rand()))
                    .take(50)
                    .map(|(block_id, output, output_id)| LedgerOutput {
                        block_id,
                        booked: MilestoneIndexTimestamp {
                            milestone_index: 0.into(),
                            milestone_timestamp: 12345.into(),
                        },
                        output,
                        output_id,
                        rent_structure: RentStructureBytes {
                            num_key_bytes: 0,
                            num_data_bytes: 100,
                        },
                    }),
            )
            .collect::<Vec<_>>();

        assert_eq!(ledger_outputs.len(), 100);

        collection
            .insert_unspent_ledger_updates(ledger_outputs.iter())
            .await
            .unwrap();

        assert_eq!(collection.count().await.unwrap(), 100);

        let mut s = collection
            .get_ledger_updates_by_address(&address, 100, None, SortOrder::Newest)
            .await
            .unwrap();

        while let Some(LedgerUpdateByAddressRecord {
            output_id,
            at,
            is_spent,
        }) = s.try_next().await.unwrap()
        {
            assert!(outputs.remove(&output_id));
            assert_eq!(
                at,
                MilestoneIndexTimestamp {
                    milestone_index: 0.into(),
                    milestone_timestamp: 12345.into()
                }
            );
            assert!(!is_spent);
        }
        assert!(outputs.is_empty());

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_ledger_updates_by_milestone() {
        let (db, collection) = setup("test-ledger-updates-by-milestone").await;

        let ctx = bee_block_stardust::protocol::protocol_parameters();

        let mut outputs = HashMap::new();
        let ledger_outputs = std::iter::repeat_with(|| (BlockId::rand(), Output::rand_basic(&ctx), OutputId::rand()))
            .take(100)
            .enumerate()
            .inspect(|(i, (_, _, output_id))| {
                assert!(outputs.insert(output_id.clone(), *i as u32 / 5).is_none());
            })
            .map(|(i, (block_id, output, output_id))| LedgerOutput {
                block_id,
                booked: MilestoneIndexTimestamp {
                    milestone_index: (i as u32 / 5).into(),
                    milestone_timestamp: (12345 + (i as u32 / 5)).into(),
                },
                output,
                output_id,
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
            })
            .collect::<Vec<_>>();

        assert_eq!(ledger_outputs.len(), 100);

        collection
            .insert_unspent_ledger_updates(ledger_outputs.iter())
            .await
            .unwrap();

        assert_eq!(collection.count().await.unwrap(), 100);

        let mut s = collection
            .get_ledger_updates_by_milestone(0.into(), 100, None)
            .await
            .unwrap();

        while let Some(LedgerUpdateByMilestoneRecord {
            output_id, is_spent, ..
        }) = s.try_next().await.unwrap()
        {
            assert_eq!(outputs.remove(&output_id), Some(0));
            assert!(!is_spent);
        }
        assert_eq!(outputs.len(), 95);

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_spent_unspent_ledger_updates() {
        let (db, collection) = setup("test-spent-unspent-ledger-updates").await;

        let ctx = bee_block_stardust::protocol::protocol_parameters();

        let mut booked_outputs = Vec::new();

        let spent_outputs = std::iter::repeat_with(|| (BlockId::rand(), Output::rand_basic(&ctx), OutputId::rand()))
            .take(100)
            .map(|(block_id, output, output_id)| LedgerOutput {
                block_id,
                booked: MilestoneIndexTimestamp {
                    milestone_index: 0.into(),
                    milestone_timestamp: 10000.into(),
                },
                output,
                output_id,
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
            })
            .inspect(|booked_output| {
                assert!(!booked_outputs.contains(booked_output));
                booked_outputs.push(booked_output.clone());
            })
            .enumerate()
            .filter_map(|(i, booked_output)| {
                if i % 2 == 0 {
                    Some(LedgerSpent {
                        output: booked_output,
                        spent_metadata: SpentMetadata {
                            transaction_id: OutputId::rand().transaction_id,
                            spent: MilestoneIndexTimestamp {
                                milestone_index: 1.into(),
                                milestone_timestamp: 20000.into(),
                            },
                        },
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        assert_eq!(booked_outputs.len(), 100);
        assert_eq!(spent_outputs.len(), 50);

        collection
            .insert_unspent_ledger_updates(booked_outputs.iter())
            .await
            .unwrap();

        collection
            .insert_spent_ledger_updates(spent_outputs.iter())
            .await
            .unwrap();

        assert_eq!(collection.count().await.unwrap(), 150);

        teardown(db).await;
    }

    async fn setup(database_name: impl ToString) -> (MongoDb, LedgerUpdateCollection) {
        let db = connect_to_test_db(database_name).await.unwrap();
        db.clear().await.unwrap();
        let collection = db.collection::<LedgerUpdateCollection>();
        collection.create_indexes().await.unwrap();
        (db, collection)
    }

    async fn teardown(db: MongoDb) {
        db.drop().await.unwrap();
    }
}
