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

/// Computes the number of addresses that hold a balance.
#[derive(Debug)]
pub struct BaseTokenActivityAnalytics;

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BaseTokenActivityAnalyticsResult {
    pub booked_value: d128,
    pub transferred_value: d128,
}

#[async_trait]
impl Analytic for BaseTokenActivityAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        db.collection::<OutputCollection>()
            .get_base_token_activity_analytics(milestone_index)
            .await
            .map(|measurement| {
                Some(Measurement::BaseTokenActivityAnalytics(PerMilestone {
                    milestone_index,
                    milestone_timestamp,
                    inner: measurement,
                }))
            })
    }
}

impl OutputCollection {
    /// Gathers output analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_base_token_activity_analytics(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<BaseTokenActivityAnalyticsResult, Error> {
        Ok(self
        .aggregate(
            vec![
                // Only consider outputs that were touched in transactions applied by this milestone.
                doc! { "$match": {
                    "$or": [
                        { "metadata.booked.milestone_index": milestone_index },
                        { "metadata.spent_metadata.spent.milestone_index": milestone_index },
                    ]
                } },
                // Group booked/spent outputs by their booking/spending transaction id and their linked address.
                // Note that outputs that are booked _and_ spent in this milestone, appear in both groups.
                doc! { "$facet": {
                    "booked_outputs": [ 
                        { "$match": { "metadata.booked.milestone_index": milestone_index } },
                        { "$group": { 
                            "_id": {
                                "tx": "$_id.transaction_id",
                                "address": "$details.address"
                            },
                            "amount": { "$sum": { "$toDecimal": "$output.amount" } },
                        } }
                    ],
                    "spent_outputs": [ 
                        { "$match": { "metadata.spent_metadata.spent.milestone_index": milestone_index } },
                        { "$group": { 
                            "_id": {
                                "tx": "$metadata.spent_metadata.transaction_id",
                                "address": "$details.address"
                            },
                            "amount": { "$sum": { "$toDecimal": "$output.amount" } },
                        } }
                    ],
                } },
                // Create a mapping between each booked output group, and all spent output groups.
                doc! { "$unwind": {
                    "path": "$booked_outputs",
                } },
                // Depending on the current booked output group address and transaction id, determine
                // if there is a spent output group with the same address. This denotes funds that
                // are sent back to an input address (and we need to account for that). 
                doc! { "$project": {
                    "booked_outputs": 1,
                    "sent_back_addr": { "$first": {
                        "$filter": {
                            "input": "$spent_outputs",
                            "as": "spent_output",
                            "cond": { "$and": [ 
                                { "$eq": ["$$spent_output._id.tx", "$booked_outputs._id.tx"] },
                                { "$eq": ["$$spent_output._id.address", "$booked_outputs._id.address"] },
                            ] }
                        }
                    } }
                } },
                // For the address of the booked output group, get the old (before the transaction) 
                // and the new (after the transaction) output amount. If that address wasn't an input
                // address, then assume a virtual input amount of 0.
                doc! { "$project": {
                    "new_amount": "$booked_outputs.amount",
                    "old_amount": { "$ifNull": ["$sent_back_addr.amount", 0] },
                } },
                // Sum amounts for various base token analytics.
                // Notes:
                // `booked_value`: Sum of all booked output amounts.
                // `transferred_value`: Sum of all (positive) deltas of amounts per transaction and address.
                //      - if funds are transferred to a _new_ output address, then the delta is equal to the 
                //        amount in that new output (due to the virtual input amount of 0);
                //      - if funds are transferred back to an input address, then the delta is the difference
                //        between the new amount and the old amount of the corresponding outputs; only if that
                //        delta is positive (i.e. funds were moved _into_ the linked address) it is counted 
                //        as a token transfer.   
                doc! { "$group": {
                    "_id": null,
                    "booked_value": { "$sum": "$new_amount" },
                    "transferred_value": { "$sum": { 
                        "$cond": [ { "$gt": [ "$new_amount", "$old_amount"] }, { "$subtract": ["$new_amount", "$old_amount"] }, 0   
                    ] } }
                } },
                doc! { "$project": {
                    "booked_value": { "$toString": "$booked_value" },
                    "transferred_value": { "$toString": "$transferred_value" },
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
