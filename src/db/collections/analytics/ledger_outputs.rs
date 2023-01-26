// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use decimal::d128;
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::Error;
use crate::{
    db::{collections::OutputCollection, MongoDbCollectionExt},
    types::tangle::MilestoneIndex,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerOutputAnalyticsResult {
    pub basic_count: u64,
    pub basic_value: u64,
    pub alias_count: u64,
    pub alias_value: u64,
    pub foundry_count: u64,
    pub foundry_value: u64,
    pub nft_count: u64,
    pub nft_value: u64,
    pub treasury_count: u64,
    pub treasury_value: u64,
}

impl OutputCollection {
    /// Gathers ledger (unspent) output analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_ledger_output_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<LedgerOutputAnalyticsResult, Error> {
        #[derive(Default, Deserialize)]
        struct Sums {
            count: u64,
            value: d128,
        }

        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Res {
            basic: Sums,
            alias: Sums,
            foundry: Sums,
            nft: Sums,
            treasury: Sums,
        }

        let res = self
            .aggregate::<Res>(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$group" : {
                        "_id": "$output.kind",
                        "count": { "$sum": 1 },
                        "value": { "$sum": { "$toDecimal": "$output.amount" } },
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "result": { "$addToSet": {
                            "k": "$_id",
                            "v": {
                                "count": "$count",
                                "value": { "$toString": "$value" },
                            }
                        } },
                    } },
                    doc! { "$replaceWith": {
                        "$arrayToObject": "$result"
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default();

        Ok(LedgerOutputAnalyticsResult {
            basic_count: res.basic.count,
            basic_value: res.basic.value.to_string().parse().unwrap(),
            alias_count: res.alias.count,
            alias_value: res.alias.value.to_string().parse().unwrap(),
            foundry_count: res.foundry.count,
            foundry_value: res.foundry.value.to_string().parse().unwrap(),
            nft_count: res.nft.count,
            nft_value: res.nft.value.to_string().parse().unwrap(),
            treasury_count: res.treasury.count,
            treasury_value: res.treasury.value.to_string().parse().unwrap(),
        })
    }
}
