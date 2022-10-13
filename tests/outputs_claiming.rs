// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {

    use chronicle::{
        db::collections::OutputCollection,
        types::{
            ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, RentStructureBytes, SpentMetadata},
            stardust::block::{
                output::{BasicOutput, OutputAmount, OutputId},
                payload::TransactionId,
                BlockId, Output,
            },
        },
    };

    use super::common::{setup_coll, setup_db, teardown};

    fn rand_output_with_value(amount: OutputAmount) -> Output {
        // We use `BasicOutput`s in the genesis.
        let mut output = BasicOutput::rand(&bee_block_stardust::protocol::protocol_parameters());
        output.amount = amount;
        Output::Basic(output)
    }

    #[tokio::test]
    async fn test_claiming() {
        let db = setup_db("test-claiming").await.unwrap();
        let output_collection = setup_coll::<OutputCollection>(&db).await.unwrap();

        let unspent_outputs = (1..=5)
            .map(|i| LedgerOutput {
                output_id: OutputId::rand(),
                rent_structure: RentStructureBytes {
                    num_key_bytes: 0,
                    num_data_bytes: 100,
                },
                output: rand_output_with_value(i.into()),
                block_id: BlockId::rand(),
                booked: MilestoneIndexTimestamp {
                    milestone_index: 0.into(),
                    milestone_timestamp: 0.into(),
                },
            })
            .collect::<Vec<_>>();

        output_collection.insert_unspent_outputs(&unspent_outputs).await.unwrap();

        let spent_outputs = unspent_outputs
            .into_iter()
            .take(4) // we spent only the first 4 outputs
            .map(|output| {
                let i = output.output.amount().0;
                LedgerSpent {
                    output,
                    spent_metadata: SpentMetadata {
                        transaction_id: TransactionId::rand(),
                        spent: MilestoneIndexTimestamp {
                            milestone_index: (i as u32).into(),
                            milestone_timestamp: (i as u32 + 10000).into(),
                        },
                    },
                }
            })
            .collect::<Vec<_>>();

        output_collection.update_spent_outputs(&spent_outputs).await.unwrap();

        let total = output_collection
            .get_claimed_token_analytics(None)
            .await
            .unwrap()
            .unwrap()
            .count;
        assert_eq!(total, (1 + 2 + 3 + 4).to_string());

        let third = output_collection
            .get_claimed_token_analytics(Some(3.into()))
            .await
            .unwrap()
            .unwrap()
            .count;
        assert_eq!(third, "3");

        teardown(db).await;
    }
}
