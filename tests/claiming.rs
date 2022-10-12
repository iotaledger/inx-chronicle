// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {

    use chronicle::{
        db::{
            collections::{OutputCollection, OutputDocument},
            MongoDbCollection,
        },
        types::{
            ledger::{LedgerOutput, MilestoneIndexTimestamp, RentStructureBytes, SpentMetadata},
            stardust::block::{
                output::{BasicOutput, OutputAmount, OutputId},
                payload::TransactionId,
                BlockId, Output,
            },
        },
    };

    use super::common::connect_to_test_db;

    fn rand_output_with_value(amount: OutputAmount) -> Output {
        // We use `BasicOutput`s in the genesis.
        let mut output = BasicOutput::rand(&bee_block_stardust::protocol::protocol_parameters());
        output.amount = amount;
        Output::Basic(output)
    }

    #[tokio::test]
    async fn test_claiming() {
        let db = connect_to_test_db("test-claiming").await.unwrap();
        db.clear().await.unwrap();
        let collection = db.collection::<OutputCollection>();
        collection.create_indexes().await.unwrap();

        let unspent_outputs = (1..=5)
            .map(|i| {
                OutputDocument::from(LedgerOutput {
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
            })
            .collect::<Vec<_>>();

        collection.insert_unspent_outputs(&unspent_outputs).await.unwrap();

        let spent_outputs = unspent_outputs
            .into_iter()
            .take(4) // we spent only the first 4 outputs
            .map(|mut output| {
                let i = output.output.amount().0;
                output.metadata.spent_metadata.replace(SpentMetadata {
                    transaction_id: TransactionId::rand(),
                    spent: MilestoneIndexTimestamp {
                        milestone_index: (i as u32).into(),
                        milestone_timestamp: (i as u32 + 10000).into(),
                    },
                });
                output
            })
            .collect::<Vec<_>>();

        collection.update_spent_outputs(&spent_outputs).await.unwrap();

        assert_eq!(collection.get_claimed_token_analytics(3.into()).await.unwrap().count, 3);

        db.drop().await.unwrap();
    }
}
