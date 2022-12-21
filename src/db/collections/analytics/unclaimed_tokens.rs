// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use decimal::d128;
use derive_more::SubAssign;
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::{Analytic, Error, Measurement, PerMilestone};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the statistics about the token claiming process.
#[derive(Default, Debug)]
pub struct UnclaimedTokenAnalytics {
    prev: Option<(MilestoneIndex, UnclaimedTokenAnalyticsResult)>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize, SubAssign)]
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
        let res = if let Some(prev) = self.prev.as_mut() {
            debug_assert!(
                milestone_index == prev.0 + 1,
                "Expected {milestone_index} found {}",
                prev.0 + 1
            );
            db.collection::<OutputCollection>()
                .update_unclaimed_token_analytics(&mut prev.1, milestone_index)
                .await?;
            *prev.0 = milestone_index.into();
            prev.1
        } else {
            self.prev
                .insert((
                    milestone_index,
                    db.collection::<OutputCollection>()
                        .get_unclaimed_token_analytics(milestone_index)
                        .await?,
                ))
                .1
        };

        Ok(Some(Measurement::UnclaimedTokenAnalytics(PerMilestone {
            milestone_index,
            milestone_timestamp,
            inner: res,
        })))
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

    /// Updates the number of claimed tokens from the previous ledger index.
    ///
    /// NOTE: The `prev_analytics` must be from `ledger_index - 1` or the results are invalid.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn update_unclaimed_token_analytics(
        &self,
        prev_analytics: &mut UnclaimedTokenAnalyticsResult,
        ledger_index: MilestoneIndex,
    ) -> Result<(), Error> {
        let claimed = self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$eq": 0 },
                        "metadata.spent_metadata.spent.milestone_index": ledger_index
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
            .unwrap_or_default();

        *prev_analytics -= claimed;

        Ok(())
    }
}
