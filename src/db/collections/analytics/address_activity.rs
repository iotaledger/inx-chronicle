// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use super::{Analytic, Measurement, PerMilestone};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the activity of addresses within a milestone.
#[derive(Debug)]
pub struct AddressActivityAnalytics;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressActivityAnalyticsResult {
    pub total_count: u64,
    pub receiving_count: u64,
    pub sending_count: u64,
}

#[async_trait]
impl Analytic for AddressActivityAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Measurement, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_address_activity_analytics(milestone_index)
            .await;
        Some(match res {
            Ok(measurement) => Ok(Measurement::AddressActivityAnalytics(PerMilestone {
                milestone_index,
                milestone_timestamp,
                inner: measurement,
            })),
            Err(err) => Err(err),
        })
    }
}

impl OutputCollection {
    /// Create aggregate statistics of all addresses.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_address_activity_analytics(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<AddressActivityAnalyticsResult, Error> {
        #[derive(Default, Deserialize)]
        struct Res {
            address_count: u64,
        }

        let (total, receiving, sending) = tokio::try_join!(
            async {
                Result::<Res, Error>::Ok(
                    self.aggregate(
                        vec![
                            doc! { "$match": {
                                "$or": [
                                    { "metadata.booked.milestone_index": milestone_index },
                                    { "metadata.spent_metadata.spent.milestone_index": milestone_index },
                                ],
                            } },
                            doc! { "$group" : { "_id": "$details.address" } },
                            doc! { "$count": "address_count" },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .unwrap_or_default(),
                )
            },
            async {
                Result::<Res, Error>::Ok(
                    self.aggregate(
                        vec![
                            doc! { "$match": {
                                "metadata.booked.milestone_index": milestone_index
                            } },
                            doc! { "$group" : { "_id": "$details.address" }},
                            doc! { "$count": "address_count" },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .unwrap_or_default(),
                )
            },
            async {
                Result::<Res, Error>::Ok(
                    self.aggregate(
                        vec![
                            doc! { "$match": {
                                "metadata.spent_metadata.spent.milestone_index": milestone_index
                            } },
                            doc! { "$group" : { "_id": "$details.address" }},
                            doc! { "$count": "address_count" },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .unwrap_or_default(),
                )
            }
        )?;
        Ok(AddressActivityAnalyticsResult {
            total_count: total.address_count,
            receiving_count: receiving.address_count,
            sending_count: sending.address_count,
        })
    }
}
