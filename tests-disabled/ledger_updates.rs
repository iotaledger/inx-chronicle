// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use std::collections::{HashMap, HashSet};

    use chronicle::{
        db::{
            mongodb::collections::{
                LedgerUpdateByAddressRecord, LedgerUpdateBySlotRecord, LedgerUpdateCollection, SortOrder,
            },
            MongoDbCollectionExt,
        },
        model::{
            ledger::{LedgerOutput, LedgerSpent, RentStructureBytes},
            metadata::SpentMetadata,
            tangle::MilestoneIndexTimestamp,
            utxo::{AddressUnlockCondition, BasicOutput, Output, OutputId},
            BlockId,
        },
    };
    use futures::TryStreamExt;
    use iota_sdk::types::block::rand::number::rand_number_range;
    use pretty_assertions::assert_eq;

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_ledger_updates_by_address() {
        let db = setup_database("test-ledger-updates-by-address").await.unwrap();
        let update_collection = setup_collection::<LedgerUpdateCollection>(&db).await.unwrap();

        let ctx = iota_sdk::types::block::protocol::protocol_parameters();

        let mut outputs = HashSet::new();
        let address_unlock_condition = AddressUnlockCondition::rand();
        let address = address_unlock_condition.address;

        let ledger_outputs = std::iter::repeat_with(|| (BlockId::rand(), rand_number_range(1..1000), OutputId::rand()))
            .take(50)
            .inspect(|(_, _, output_id)| {
                outputs.insert(*output_id);
            })
            .map(|(block_id, amount, output_id)| LedgerOutput {
                block_id,
                booked: MilestoneIndexTimestamp {
                    milestone_index: 0.into(),
                    milestone_timestamp: 12345.into(),
                },
                output: Output::Basic(BasicOutput {
                    amount: amount.into(),
                    native_tokens: Vec::new().into_boxed_slice(),
                    address_unlock_condition,
                    storage_deposit_return_unlock_condition: None,
                    timelock_unlock_condition: None,
                    expiration_unlock_condition: None,
                    features: Vec::new().into_boxed_slice(),
                }),
                output_id,
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
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

        assert_eq!(outputs.len(), 50);
        assert_eq!(ledger_outputs.len(), 100);

        update_collection
            .insert_unspent_ledger_updates(ledger_outputs.iter())
            .await
            .unwrap();

        assert_eq!(update_collection.count().await.unwrap(), 100);

        let mut s = update_collection
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
        let db = setup_database("test-ledger-updates-by-milestone").await.unwrap();
        let update_collection = setup_collection::<LedgerUpdateCollection>(&db).await.unwrap();

        let ctx = iota_sdk::types::block::protocol::protocol_parameters();

        let mut outputs = HashMap::new();
        let ledger_outputs = std::iter::repeat_with(|| (BlockId::rand(), Output::rand_basic(&ctx), OutputId::rand()))
            .take(100)
            .enumerate()
            .inspect(|(i, (_, _, output_id))| {
                assert!(outputs.insert(*output_id, *i as u32 / 5).is_none());
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

        update_collection
            .insert_unspent_ledger_updates(ledger_outputs.iter())
            .await
            .unwrap();

        assert_eq!(update_collection.count().await.unwrap(), 100);

        let mut s = update_collection
            .get_ledger_updates_by_milestone(0.into(), 100, None)
            .await
            .unwrap();

        while let Some(LedgerUpdateBySlotRecord {
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
    async fn test_insert_spent_ledger_updates() {
        let db = setup_database("test-insert-spent-ledger-updates").await.unwrap();
        let update_collection = setup_collection::<LedgerUpdateCollection>(&db).await.unwrap();

        let ctx = iota_sdk::types::block::protocol::protocol_parameters();

        let unspent_outputs = std::iter::repeat_with(|| (BlockId::rand(), Output::rand_basic(&ctx), OutputId::rand()))
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
            .collect::<Vec<_>>();

        update_collection
            .insert_unspent_ledger_updates(unspent_outputs.iter())
            .await
            .unwrap();

        assert_eq!(update_collection.count().await.unwrap(), 100);

        let spent_outputs = unspent_outputs
            .into_iter()
            .enumerate()
            .filter_map(|(i, output)| {
                if i % 2 == 0 {
                    Some(LedgerSpent {
                        output,
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

        update_collection
            .insert_spent_ledger_updates(spent_outputs.iter())
            .await
            .unwrap();

        assert_eq!(update_collection.count().await.unwrap(), 150);

        teardown(db).await;
    }
}
