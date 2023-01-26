// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::{Analytic, Error, Measurement, PerMilestone};
use crate::{
    db::{collections::BlockCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the byte size of a milestone.
#[derive(Debug)]
pub struct MilestoneSizeAnalytics;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MilestoneSizeAnalyticsResult {
    // pub total_bytes: u64,
    pub total_transaction_payload_bytes: u64,
    pub total_tagged_data_payload_bytes: u64,
    pub total_milestone_payload_bytes: u64,
    pub total_treasury_transaction_payload_bytes: u64,
    pub total_milestone_bytes: u64,
}

#[async_trait]
impl Analytic for MilestoneSizeAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        let total_milestone_bytes = db
            .collection::<BlockCollection>()
            .get_milestone_size_analytics(milestone_index)
            .await?;

        Ok(Some(Measurement::MilestoneSizeAnalytics(PerMilestone {
            milestone_index,
            milestone_timestamp,
            inner: total_milestone_bytes,
        })))
    }
}

impl BlockCollection {
    /// Gathers milestone byte size analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_milestone_size_analytics(
        &self,
        index: MilestoneIndex,
    ) -> Result<MilestoneSizeAnalyticsResult, Error> {
        // #[derive(Deserialize)]
        // struct MilestoneSizeResult {
        //     total_transaction_payload_bytes: u64,
        //     total_tagged_data_payload_bytes: u64,
        //     total_milestone_payload_bytes: u64,
        //     total_treasury_transaction_payload_bytes: u64,
        //     total_milestone_bytes: u64,
        // }

        let res = self
            .aggregate::<MilestoneSizeAnalyticsResult>(
                vec![
                    doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
                    doc! { "$group" : {
                        "_id": "$block.payload.kind",
                        "num_bytes": { "$sum": { "$size": "$raw" } },
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "total_transaction_payload_bytes": { "$sum": { "$cond": [ { "$eq": [ "$_id", "transaction" ] }, "$num_bytes", 0] } },
                        "total_tagged_data_payload_bytes": { "$sum": { "$cond": [ { "$eq": [ "$_id", "tagged_data" ] }, "$num_bytes", 0] } },
                        "total_milestone_payload_bytes": { "$sum": { "$cond": [ { "$eq": [ "$_id", "milestone" ] }, "$num_bytes", 0] } },
                        "total_treasury_transaction_payload_bytes": { "$sum": { "$cond": [ { "$eq": [ "$_id", "treasury_transaction" ] }, "$num_bytes", 0] } },
                        "total_milestone_bytes": { "$sum": "$num_bytes" },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?;

        Ok(res
            // .map(|res| MilestoneSizeAnalyticsResult {
            //     total_milestone_bytes: res.total_milestone_bytes,
            // })
            .unwrap_or_default())
    }
}
