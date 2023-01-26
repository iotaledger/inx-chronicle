// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::Error;
use crate::{
    db::{
        collections::{OutputCollection, ProtocolUpdateCollection},
        MongoDbCollection, MongoDbCollectionExt,
    },
    types::{
        ledger::RentStructureBytes,
        tangle::{MilestoneIndex, RentStructure},
    },
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerSizeAnalyticsResult {
    pub total_key_bytes: u64,
    pub total_data_bytes: u64,
    pub total_storage_deposit_value: u64,
}

impl OutputCollection {
    /// Gathers byte cost and storage deposit analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_ledger_size_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<LedgerSizeAnalyticsResult, Error> {
        #[derive(Deserialize)]
        struct Res {
            total_key_bytes: String,
            total_data_bytes: String,
            rent_structure: RentStructure,
        }

        let res = self
            .aggregate::<Res>(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "total_key_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_key_bytes" } },
                        "total_data_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_data_bytes" } },
                    } },
                    doc! { "$lookup": {
                        "from": ProtocolUpdateCollection::NAME,
                        "pipeline": [
                            { "$match": { "_id": { "$lte": ledger_index } } },
                            { "$sort": { "_id": -1 } },
                            { "$limit": 1 },
                            { "$replaceWith": "$parameters.rent_structure" }
                        ],
                        "as": "rent_structure",
                    } },
                    doc! { "$project": {
                        "total_key_bytes": { "$toString": "$total_key_bytes" },
                        "total_data_bytes": { "$toString": "$total_data_bytes" },
                        "rent_structure": { "$first": "$rent_structure" },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?;

        Ok(res
            .map(|res| {
                let rent_structure_bytes = RentStructureBytes {
                    num_key_bytes: res.total_key_bytes.parse().unwrap(),
                    num_data_bytes: res.total_data_bytes.parse().unwrap(),
                };

                LedgerSizeAnalyticsResult {
                    total_key_bytes: rent_structure_bytes.num_key_bytes,
                    total_data_bytes: rent_structure_bytes.num_data_bytes,
                    total_storage_deposit_value: rent_structure_bytes.rent_cost(&res.rent_structure.into()),
                }
            })
            .unwrap_or_default())
    }
}
