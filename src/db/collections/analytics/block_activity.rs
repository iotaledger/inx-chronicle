// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::Error;
use crate::{
    db::{collections::BlockCollection, MongoDbCollectionExt},
    types::tangle::MilestoneIndex,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct PayloadActivityAnalyticsResult {
    pub transaction_count: u32,
    pub treasury_transaction_count: u32,
    pub milestone_count: u32,
    pub tagged_data_count: u32,
    pub no_payload_count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TransactionActivityAnalyticsResult {
    pub confirmed_count: u32,
    pub conflicting_count: u32,
    pub no_transaction_count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockActivityAnalyticsResult {
    pub payload: PayloadActivityAnalyticsResult,
    pub transaction: TransactionActivityAnalyticsResult,
}

impl BlockCollection {
    /// Gathers past-cone payload activity statistics for a given milestone.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_block_activity_analytics(
        &self,
        index: MilestoneIndex,
    ) -> Result<BlockActivityAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
                    doc! { "$group": {
                        "_id": null,
                        "transaction_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$block.payload.kind", "transaction" ] }, 1 , 0 ]
                        } },
                        "treasury_transaction_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$block.payload.kind", "treasury_transaction" ] }, 1 , 0 ]
                        } },
                        "milestone_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$block.payload.kind", "milestone" ] }, 1 , 0 ]
                        } },
                        "tagged_data_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$block.payload.kind", "tagged_data" ] }, 1 , 0 ]
                        } },
                        "no_payload_count": { "$sum": {
                            "$cond": [ { "$not": "$block.payload" }, 1 , 0 ]
                        } },
                        "confirmed_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$metadata.inclusion_state", "included" ] }, 1 , 0 ]
                        } },
                        "conflicting_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$metadata.inclusion_state", "conflicting" ] }, 1 , 0 ]
                        } },
                        "no_transaction_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$metadata.inclusion_state", "no_transaction" ] }, 1 , 0 ]
                        } },
                    } },
                    doc! { "$project": {
                        "payload": {
                            "transaction_count": "$transaction_count",
                            "treasury_transaction_count": "$treasury_transaction_count",
                            "milestone_count": "$milestone_count",
                            "tagged_data_count": "$tagged_data_count",
                            "no_payload_count": "$no_payload_count",
                        },
                        "transaction": {
                            "confirmed_count": "$confirmed_count",
                            "conflicting_count": "$conflicting_count",
                            "no_transaction_count": "$no_transaction_count",
                        }
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }
}
