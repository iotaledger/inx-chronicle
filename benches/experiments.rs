// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![feature(test)]

extern crate test;

use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use chronicle::{
    db::{
        collections::{BlockCollection, OutputCollection, ProtocolUpdateCollection},
        MongoDb, MongoDbConfig,
    },
    types::{
        context::TryFromWithContext,
        ledger::{BlockMetadata, LedgerInclusionState, RentStructureBytes},
        stardust::block::{payload::TransactionEssence, Address, Block, Input, Output, Payload},
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};
use derive_more::{AddAssign, SubAssign};
use futures::{StreamExt, TryStreamExt};
use iota_types::block::output::Rent;

trait StupidRent {
    fn ledger_stat(&self, protocol_params: &ProtocolParameters) -> LedgerSizeStatistic;
}

impl StupidRent for Output {
    fn ledger_stat(&self, protocol_params: &ProtocolParameters) -> LedgerSizeStatistic {
        let config = iota_types::block::protocol::ProtocolParameters::try_from(protocol_params.clone()).unwrap();
        let output = iota_types::block::output::Output::try_from_with_context(&config, self.clone()).unwrap();
        let rent_bytes = RentStructureBytes::compute(&output);
        LedgerSizeStatistic {
            total_storage_deposit_value: output.rent_cost(config.rent_structure()),
            total_key_bytes: rent_bytes.num_key_bytes,
            total_data_bytes: rent_bytes.num_data_bytes,
        }
    }
}

pub trait BlockAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, index: MilestoneIndex);
    fn handle_block(&mut self, block: &Block, block_metadata: &BlockMetadata, inputs: &Option<Vec<Output>>);
    fn end_milestone(&mut self, index: MilestoneIndex);
}

#[derive(Clone, Debug, Default, AddAssign, SubAssign)]
struct LedgerSizeStatistic {
    total_storage_deposit_value: u64,
    total_key_bytes: u64,
    total_data_bytes: u64,
}

#[derive(Debug, Default)]
struct LedgerSizeAnalytics {
    protocol_params: Option<ProtocolParameters>,
    stats: LedgerSizeStatistic,
}

impl BlockAnalytics for LedgerSizeAnalytics {
    type Measurement = LedgerSizeStatistic;

    fn begin_milestone(&mut self, _index: MilestoneIndex) {}

    fn handle_block(&mut self, block: &Block, _block_metadata: &BlockMetadata, inputs: &Option<Vec<Output>>) {
        let protocol_params = self.protocol_params.as_ref().unwrap();
        if let Some(payload) = block.payload.as_ref() {
            match payload {
                Payload::Transaction(txn) => {
                    let TransactionEssence::Regular { outputs, .. } = &txn.essence;
                    for output in outputs.iter() {
                        self.stats += output.ledger_stat(protocol_params);
                    }
                }
                _ => (),
            }
        }
        if let Some(inputs) = inputs {
            for input in inputs {
                self.stats -= input.ledger_stat(protocol_params);
            }
        }
    }

    fn end_milestone(&mut self, index: MilestoneIndex) {
        println!("Milestone {}: {:?}", index, self.stats);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TransactionStatistic {
    confirmed_count: u32,
    conflicting_count: u32,
    no_transaction_count: u32,
}

#[derive(Clone, Debug, Default)]
struct TransactionAnalytics(TransactionStatistic);

impl BlockAnalytics for TransactionAnalytics {
    type Measurement = TransactionStatistic;

    fn begin_milestone(&mut self, _index: MilestoneIndex) {
        self.0 = TransactionStatistic::default();
    }

    fn handle_block(&mut self, _block: &Block, block_metadata: &BlockMetadata, _inputs: &Option<Vec<Output>>) {
        match block_metadata.inclusion_state {
            LedgerInclusionState::Conflicting => self.0.conflicting_count += 1,
            LedgerInclusionState::Included => self.0.confirmed_count += 1,
            LedgerInclusionState::NoTransaction => self.0.no_transaction_count += 1,
        }
    }

    fn end_milestone(&mut self, index: MilestoneIndex) {
        println!("Milestone {}: {:?}", index, self.0);
    }
}

#[derive(Debug, Default)]
struct AddressBalanceAnalytics {
    stats: AddressBalanceStatistic,
    addresses: Arc<Mutex<HashMap<Address, u64>>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AddressBalanceStatistic {
    address_with_balance_count: u64,
}

impl BlockAnalytics for AddressBalanceAnalytics {
    type Measurement = AddressBalanceStatistic;

    fn begin_milestone(&mut self, _index: MilestoneIndex) {
        self.stats = AddressBalanceStatistic::default();
    }

    fn handle_block(&mut self, block: &Block, _block_metadata: &BlockMetadata, inputs: &Option<Vec<Output>>) {
        if let Some(payload) = block.payload.as_ref() {
            match payload {
                Payload::Transaction(txn) => {
                    let TransactionEssence::Regular { outputs, .. } = &txn.essence;
                    for output in outputs.iter() {
                        if let Some(address) = output.owning_address() {
                            *self.addresses.lock().unwrap().entry(*address).or_default() += output.amount().0;
                        }
                    }
                }
                _ => (),
            }
        }
        if let Some(inputs) = inputs {
            for input in inputs {
                if let Some(address) = input.owning_address() {
                    *self.addresses.lock().unwrap().entry(*address).or_default() -= input.amount().0;
                }
            }
        }
    }

    fn end_milestone(&mut self, index: MilestoneIndex) {
        println!("Milestone {}: {:?}", index, self.stats);
    }
}

async fn experiment(bencher: &mut test::Bencher) -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let config = MongoDbConfig {
        conn_str:
            "mongodb://dev-chronicle:password@localhost:27017/?authSource=admin&replicaSet=dbrs&directConnection=true"
                .to_string(),
        database_name: "chronicle_beta_25".to_string(),
    };

    let db = MongoDb::connect(&config).await?;
    let block_collection = db.collection::<BlockCollection>();
    let output_collection = db.collection::<OutputCollection>();

    // Get all the input data from the db before we start benchmarking anything
    let mut input_data = Vec::new();
    for milestone in 1000..1100 {
        println!("Gathering data from milestone {milestone}");
        let milestone = MilestoneIndex::from(milestone);
        let protocol_params = db
            .collection::<ProtocolUpdateCollection>()
            .get_protocol_parameters_for_ledger_index(milestone)
            .await?
            .unwrap()
            .parameters;
        let output_collection = &output_collection;
        let cone = block_collection
            .get_referenced_blocks_in_white_flag_order_stream(milestone)
            .await?
            .and_then(|(block, metadata)| async move {
                let mut input_res = None;
                if let Some(payload) = block.payload.as_ref() {
                    match payload {
                        Payload::Transaction(txn) => {
                            let TransactionEssence::Regular { inputs, .. } = &txn.essence;
                            input_res = Some(
                                futures::stream::iter(inputs.iter().filter_map(|input| match input {
                                    Input::Utxo(output_id) => Some(*output_id),
                                    _ => None,
                                }))
                                .then(|output_id| async move {
                                    Result::<_, mongodb::error::Error>::Ok(
                                        output_collection.get_output(&output_id).await?.unwrap(),
                                    )
                                })
                                .try_collect::<Vec<_>>()
                                .await?,
                            );
                        }
                        _ => (),
                    }
                }
                Ok((block, metadata, input_res))
            })
            .try_collect::<Vec<_>>()
            .await?;
        input_data.push((milestone, protocol_params, cone));
    }

    bencher.iter(|| {
        let mut ledger_size_analytics = LedgerSizeAnalytics::default();
        let mut txn_analytics = TransactionAnalytics::default();
        let mut address_balance_analytics = AddressBalanceAnalytics::default();

        for (milestone, protocol_params, cone) in &input_data {
            ledger_size_analytics.protocol_params = Some(protocol_params.clone());
            // BENCH
            ledger_size_analytics.begin_milestone(*milestone);
            txn_analytics.begin_milestone(*milestone);
            address_balance_analytics.begin_milestone(*milestone);
            for (block, metadata, inputs) in cone {
                ledger_size_analytics.handle_block(&block, &metadata, &inputs);
                txn_analytics.handle_block(&block, &metadata, &inputs);
                address_balance_analytics.handle_block(&block, &metadata, &inputs);
            }
            ledger_size_analytics.end_milestone(*milestone);
            txn_analytics.end_milestone(*milestone);
            address_balance_analytics.end_milestone(*milestone);
        }
    });

    Ok(())
}

#[bench]
fn bench_test(bencher: &mut test::Bencher) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            experiment(bencher).await.unwrap();
        })
}
