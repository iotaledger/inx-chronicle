// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use decimal::d128;
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::{Analytic, Error, Measurement, PerMilestone};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the statistics about the token claiming process.
#[derive(Debug)]
pub struct UnclaimedTokenAnalytics;

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UnclaimedTokenAnalyticsResult {
    pub unclaimed_count: u64,
    pub unclaimed_value: d128,
}

#[async_trait]
impl Analytic for UnclaimedTokenAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        db.collection::<OutputCollection>()
            .get_unclaimed_token_analytics(milestone_index)
            .await
            .map(|measurement| {
                Some(Measurement::UnclaimedTokenAnalytics(PerMilestone {
                    milestone_index,
                    milestone_timestamp,
                    inner: measurement,
                }))
            })
    }
}

impl OutputCollection {
    /// Gets the number of claimed tokens.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_unclaimed_token_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<UnclaimedTokenAnalyticsResult, Error> {
        Ok(self
            .aggregate(
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
            .unwrap_or_default())
    }
}
