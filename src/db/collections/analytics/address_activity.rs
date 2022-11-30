// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use influxdb::InfluxDbWriteable;
use crate::{db::{MongoDb, collections::OutputCollection, MongoDbCollectionExt}, types::{tangle::MilestoneIndex, stardust::milestone::MilestoneTimestamp}};
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use super::{Analytic, Measurement, PerMilestone};

#[derive(Debug)]
pub struct AddressActivityAnalytics;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
struct AddressActivityAnalyticsResult {
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
    ) -> Option<Result<Box<dyn Measurement>, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_address_activity_analytics(milestone_index)
            .await;
        Some(match res {
            Ok(measurement) => Ok(Box::new(PerMilestone {
                milestone_index,
                milestone_timestamp,
                measurement,
            })),
            Err(err) => Err(err),
        })
    }
}

impl OutputCollection {

    /// TODO: Merge with above
/// Create aggregate statistics of all addresses.
#[tracing::instrument(skip(self), err, level = "trace")]
async fn get_address_activity_analytics(
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

impl Measurement for PerMilestone<AddressActivityAnalyticsResult> {
    fn into_write_query(&self) -> influxdb::WriteQuery {
        influxdb::Timestamp::from(self.milestone_timestamp)
            .into_query("stardust_address_activity")
            .add_field("milestone_index", self.milestone_index)
            .add_field("total_count", self.measurement.total_count)
            .add_field("receiving_count", self.measurement.receiving_count)
            .add_field("sending_count", self.measurement.sending_count)
    }
}
