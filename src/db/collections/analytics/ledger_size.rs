// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use derive_more::{AddAssign, SubAssign};
use iota_types::block::output::Rent;

use super::{Analytic, BlockAnalytics, Error, Measurement, PerMilestone};
use crate::{
    db::MongoDb,
    types::{
        ledger::{BlockMetadata, RentStructureBytes},
        stardust::{
            block::{payload::TransactionEssence, Block, Output, Payload},
            milestone::MilestoneTimestamp,
        },
        tangle::{MilestoneIndex, ProtocolParameters}, context::TryFromWithContext,
    },
};

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

/// Computes the size of the ledger.
#[derive(Debug, Default)]
pub struct LedgerSizeAnalytics {
    protocol_params: Option<ProtocolParameters>,
    stats: LedgerSizeStatistic,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
pub struct LedgerSizeStatistic {
    pub total_key_bytes: u64,
    pub total_data_bytes: u64,
    pub total_storage_deposit_value: u64,
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

#[async_trait]
impl Analytic for LedgerSizeAnalytics {
    async fn get_measurement(
        &mut self,
        _db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        Ok(Some(Measurement::LedgerSizeAnalytics(PerMilestone {
            milestone_index,
            milestone_timestamp,
            inner: self.stats,
        })))
    }
}

// #[async_trait]
// impl Analytic for LedgerSizeAnalytics {
//     async fn get_measurement(
//         &mut self,
//         db: &MongoDb,
//         milestone_index: MilestoneIndex,
//         milestone_timestamp: MilestoneTimestamp,
//     ) -> Result<Option<Measurement>, Error> {
//         db.collection::<OutputCollection>()
//             .get_ledger_size_analytics(milestone_index)
//             .await
//             .map(|measurement| {
//                 Some(Measurement::LedgerSizeAnalytics(PerMilestone {
//                     milestone_index,
//                     milestone_timestamp,
//                     inner: measurement,
//                 }))
//             })
//     }
// }

// impl OutputCollection {
//     /// Gathers byte cost and storage deposit analytics.
//     #[tracing::instrument(skip(self), err, level = "trace")]
//     pub async fn get_ledger_size_analytics(
//         &self,
//         ledger_index: MilestoneIndex,
//     ) -> Result<LedgerSizeStatistic, Error> {
//         #[derive(Deserialize)]
//         struct Res {
//             total_key_bytes: String,
//             total_data_bytes: String,
//             rent_structure: RentStructure,
//         }

//         let res = self
//             .aggregate::<Res>(
//                 vec![
//                     doc! { "$match": {
//                         "metadata.booked.milestone_index": { "$lte": ledger_index },
//                         "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
//                     } },
//                     doc! { "$group" : {
//                         "_id": null,
//                         "total_key_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_key_bytes" } },
//                         "total_data_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_data_bytes" } },
//                     } },
//                     doc! { "$lookup": {
//                         "from": ProtocolUpdateCollection::NAME,
//                         "pipeline": [
//                             { "$match": { "_id": { "$lte": ledger_index } } },
//                             { "$sort": { "_id": -1 } },
//                             { "$limit": 1 },
//                             { "$replaceWith": "$parameters.rent_structure" }
//                         ],
//                         "as": "rent_structure",
//                     } },
//                     doc! { "$project": {
//                         "total_key_bytes": { "$toString": "$total_key_bytes" },
//                         "total_data_bytes": { "$toString": "$total_data_bytes" },
//                         "rent_structure": { "$first": "$rent_structure" },
//                     } },
//                 ],
//                 None,
//             )
//             .await?
//             .try_next()
//             .await?;

//         Ok(res
//             .map(|res| {
//                 let rent_structure_bytes = RentStructureBytes {
//                     num_key_bytes: res.total_key_bytes.parse().unwrap(),
//                     num_data_bytes: res.total_data_bytes.parse().unwrap(),
//                 };

//                 LedgerSizeStatistic {
//                     total_key_bytes: rent_structure_bytes.num_key_bytes,
//                     total_data_bytes: rent_structure_bytes.num_data_bytes,
//                     total_storage_deposit_value: rent_structure_bytes.rent_cost(&res.rent_structure.into()),
//                 }
//             })
//             .unwrap_or_default())
//     }
// }
