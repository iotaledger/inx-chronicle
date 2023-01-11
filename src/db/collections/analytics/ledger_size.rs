// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use iota_types::block::output::{Rent, RentStructureBuilder};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::{Analytic, Error, Measurement, PerMilestone};
use crate::{
    db::{
        collections::{OutputCollection, ProtocolUpdateCollection},
        MongoDb, MongoDbCollection, MongoDbCollectionExt,
    },
    types::{
        stardust::milestone::MilestoneTimestamp,
        tangle::{MilestoneIndex, RentStructure},
    },
};

/// Computes the size of the ledger.
#[derive(Debug)]
pub struct LedgerSizeAnalytics;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LedgerSizeAnalyticsResult {
    pub total_storage_deposit_value: u64,
    pub total_key_bytes: u64,
    pub total_data_bytes: u64,
    pub total_byte_cost: u64,
}

#[async_trait]
impl Analytic for LedgerSizeAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        db.collection::<OutputCollection>()
            .get_ledger_size_analytics(milestone_index)
            .await
            .map(|measurement| {
                Some(Measurement::LedgerSizeAnalytics(PerMilestone {
                    milestone_index,
                    milestone_timestamp,
                    inner: measurement,
                }))
            })
    }
}

impl OutputCollection {
    /// Gathers byte cost and storage deposit analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_ledger_size_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<LedgerSizeAnalyticsResult, Error> {
        #[derive(Default, Deserialize)]
        struct Result {
            total_storage_deposit_value: String,
            total_key_bytes: String,
            total_data_bytes: String,
            rent_structure: Option<RentStructure>,
        }

        impl Rent for Result {
            fn weighted_bytes(&self, config: &iota_types::block::output::RentStructure) -> u64 {
                let (total_key_bytes, total_data_bytes) = (
                    self.total_key_bytes.parse::<u64>().unwrap(),
                    self.total_data_bytes.parse::<u64>().unwrap(),
                );

                (total_key_bytes * config.byte_factor_key() as u64)
                    + (total_data_bytes * config.byte_factor_data() as u64)
            }
        }

        let res = self
            .aggregate::<Result>(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "total_key_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_key_bytes" } },
                        "total_data_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_data_bytes" } },
                        "total_storage_deposit_value": { "$sum": { "$toDecimal": "$output.storage_deposit_return_unlock_condition.amount" } },
                    } },
                    doc! { "$lookup": {
                        "from": ProtocolUpdateCollection::NAME,
                        "pipeline": [
                            { "$match": { "_id": { "$lte": ledger_index } } },
                            { "$limit": 1 },
                            { "$replaceWith": "$parameters.rent_structure" }
                        ],
                        "as": "rent_structure",
                    } },
                    doc! { "$project": {
                        "total_storage_deposit_value": { "$toString": "$total_storage_deposit_value" },
                        "total_key_bytes": { "$toString": "$total_key_bytes" },
                        "total_data_bytes": { "$toString": "$total_data_bytes" },
                        "rent_structure": { "$first": "$rent_structure" },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default();

        Ok(LedgerSizeAnalyticsResult {
            total_storage_deposit_value: res.total_storage_deposit_value.parse().unwrap(),
            total_key_bytes: res.total_key_bytes.parse().unwrap(),
            total_data_bytes: res.total_data_bytes.parse().unwrap(),
            total_byte_cost: res
                .rent_structure
                .map(|rs| {
                    res.rent_cost(
                        &RentStructureBuilder::new()
                            .byte_cost(rs.v_byte_cost)
                            .byte_factor_data(rs.v_byte_factor_data)
                            .byte_factor_key(rs.v_byte_factor_key)
                            .finish(),
                    )
                })
                .unwrap_or_default(),
        })
    }
}
