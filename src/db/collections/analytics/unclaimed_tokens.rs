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
pub struct UnclaimedTokenAnalyticsResult {
    pub unclaimed_count: u64,
    pub unclaimed_value: u64,
}

impl OutputCollection {
    /// Gets the number of claimed tokens.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_unclaimed_token_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<UnclaimedTokenAnalyticsResult, Error> {
        #[derive(Deserialize)]
        struct Res {
            unclaimed_count: u64,
            unclaimed_value: d128,
        }

        impl From<Res> for UnclaimedTokenAnalyticsResult {
            fn from(value: Res) -> Self {
                Self {
                    unclaimed_count: value.unclaimed_count,
                    unclaimed_value: value.unclaimed_value.to_string().parse().unwrap(),
                }
            }
        }

        Ok(self
            .aggregate::<Res>(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$eq": 0 },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$group": {
                        "_id": null,
                        "unclaimed_count": { "$sum": 1 },
                        "unclaimed_value": { "$sum": { "$toDecimal": "$output.amount" } },
                    } },
                    doc! { "$project": {
                        "unclaimed_count": 1,
                        "unclaimed_value": { "$toString": "$unclaimed_value" },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(Into::into)
            .unwrap_or_default())
    }
}
