// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use super::{Analytic, Error, Measurement, TimeInterval};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the activity of addresses within a milestone.
#[derive(Debug, Default)]
pub struct DailyActiveAddressesAnalytics {
    current_date: Option<Date>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyActiveAddressAnalyticsResult {
    pub count: u64,
}

#[async_trait]
impl Analytic for DailyActiveAddressesAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        _: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        let incoming_date = OffsetDateTime::try_from(milestone_timestamp)?.date();

        if self.current_date.is_none() {
            self.current_date = Some(incoming_date);
        }

        if let Some(current_date) = self.current_date {
            if current_date.next_day() == Some(incoming_date) {
                let from = current_date.midnight().assume_utc();
                let to_exclusive = incoming_date.midnight().assume_utc();

                let res = db
                    .collection::<OutputCollection>()
                    .get_interval_address_activity_analytics(from.into(), to_exclusive.into())
                    .await
                    .map(|measurement| {
                        Some(Measurement::DailyActiveAddressAnalytics(TimeInterval {
                            from,
                            to_exclusive,
                            inner: measurement,
                        }))
                    });

                self.current_date = Some(incoming_date);
                return res;
            }
        }

        Ok(None)
    }
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
