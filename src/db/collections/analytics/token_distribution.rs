// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::bson::doc;
use time::{Date, OffsetDateTime};

use super::{Analytic, Error, Measurement, TimeInterval};
use crate::{
    db::{
        collections::{outputs::TokenDistribution, OutputCollection},
        MongoDb, MongoDbCollectionExt,
    },
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the token distribution in certain time intervals.
#[derive(Debug, Default)]
pub struct TokenDistributionAnalytics {
    current_date: Option<Date>,
}

#[async_trait]
impl Analytic for TokenDistributionAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        _: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        let incoming_date = OffsetDateTime::try_from(milestone_timestamp)?.date();

        let current_date = self.current_date.get_or_insert(incoming_date);

        if current_date.next_day() == Some(incoming_date) {
            let from = current_date.midnight().assume_utc();
            let to_exclusive = incoming_date.midnight().assume_utc();

            let res = db
                .collection::<OutputCollection>()
                .get_token_distribution_per_interval(from.into(), to_exclusive.into())
                .await
                .map(|measurement| {
                    Some(Measurement::TokenDistributionAnalytics(TimeInterval {
                        from,
                        to_exclusive,
                        inner: measurement,
                    }))
                });

            *current_date = incoming_date;
            return res;
        }

        Ok(None)
    }
}

impl OutputCollection {
    /// Create token distribution statistics per time interval.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_token_distribution_per_interval(
        &self,
        from: MilestoneTimestamp,
        to: MilestoneTimestamp,
    ) -> Result<TokenDistribution, Error> {
        let distribution = self
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
                    doc! { "$group" : {
                        "_id": "$details.address",
                        "balance": { "$sum": { "$toDecimal": "$output.amount" } },
                    } },
                    doc! { "$set": { "index": { "$toInt": { "$log10": "$balance" } } } },
                    doc! { "$group" : {
                        "_id": "$index",
                        "address_count": { "$sum": 1 },
                        "total_balance": { "$sum": "$balance" },
                    } },
                    doc! { "$sort": { "_id": 1 } },
                    doc! { "$project": {
                        "_id": 0,
                        "index": "$_id",
                        "address_count": 1,
                        "total_balance": { "$toString": "$total_balance" },
                    } },
                ],
                None,
            )
            .await?
            .try_collect()
            .await?;
        Ok(TokenDistribution { distribution })
    }
}
