// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::Error;
use crate::{
    db::{collections::OutputCollection, MongoDbCollectionExt},
    types::stardust::milestone::MilestoneTimestamp,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct DailyActiveAddressAnalyticsResult {
    pub count: u64,
}

impl OutputCollection {
    /// Create active address statistics for a given time interval `[from,to)`.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_interval_address_activity_analytics(
        &self,
        from: MilestoneTimestamp,
        to: MilestoneTimestamp,
    ) -> Result<DailyActiveAddressAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": { "$or": [
                        { "$and": [
                            { "metadata.booked.milestone_timestamp": { "$gte": from } },
                            { "metadata.booked.milestone_timestamp": { "$lt": to } },
                        ] },
                        { "$and": [
                            { "metadata.spent_metadata.spent.milestone_timestamp": { "$gte": from } },
                            { "metadata.spent_metadata.spent.milestone_timestamp": { "$lt": to } }
                        ] },
                    ] } },
                    doc! { "$group": { "_id": { "addr": "$details.address" } } },
                    doc! { "$count": "count" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }
}
