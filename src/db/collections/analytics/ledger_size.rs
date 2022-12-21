// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use decimal::d128;
use derive_more::{AddAssign, SubAssign};
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::{Analytic, Error, Measurement, PerMilestone};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{
        stardust::milestone::MilestoneTimestamp,
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

/// Computes the size of the ledger.
#[derive(Debug, Default)]
pub struct LedgerSizeAnalytics {
    prev: Option<(MilestoneIndex, LedgerSizeAnalyticsResult)>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize, AddAssign, SubAssign)]
pub struct LedgerSizeAnalyticsResult {
    pub total_storage_deposit_value: d128,
    pub total_key_bytes: d128,
    pub total_data_bytes: d128,
}

impl LedgerSizeAnalyticsResult {
    pub fn total_byte_cost(&self, protocol_params: &ProtocolParameters) -> d128 {
        let rent_structure = protocol_params.rent_structure;
        d128::from(rent_structure.v_byte_cost)
            * ((self.total_key_bytes * d128::from(rent_structure.v_byte_factor_key as u32))
                + (self.total_data_bytes * d128::from(rent_structure.v_byte_factor_data as u32)))
    }
}

#[async_trait]
impl Analytic for LedgerSizeAnalytics {
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
                .update_ledger_size_analytics(&mut prev.1, milestone_index)
                .await?;
            *prev.0 = milestone_index.into();
            prev.1
        } else {
            self.prev
                .insert((
                    milestone_index,
                    db.collection::<OutputCollection>()
                        .get_ledger_size_analytics(milestone_index)
                        .await?,
                ))
                .1
        };

        Ok(Some(Measurement::LedgerSizeAnalytics(PerMilestone {
            milestone_index,
            milestone_timestamp,
            inner: res,
        })))
    }
}

impl OutputCollection {
    /// Gathers byte cost and storage deposit analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_ledger_size_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<LedgerSizeAnalyticsResult, Error> {
        Ok(self
        .aggregate(
            vec![
                doc! { "$match": {
                    "metadata.booked.milestone_index": { "$lte": ledger_index },
                    "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                } },
                doc! { "$group" : {
                    "_id": null,
                    "total_key_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_key_bytes" } },
                    "total_data_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_data_bytes" } },
                    "total_storage_deposit_value": { "$sum": { "$toDecimal": "$output.storage_deposit_return_unlock_condition.amount" } },
                } },
                doc! { "$project": {
                    "total_storage_deposit_value": { "$toString": "$total_storage_deposit_value" },
                    "total_key_bytes": { "$toString": "$total_key_bytes" },
                    "total_data_bytes": { "$toString": "$total_data_bytes" },
                } },
            ],
            None,
        )
        .await?
        .try_next()
        .await?
        .unwrap_or_default())
    }

    /// Gathers byte cost and storage deposit analytics and updates the analytics from the previous ledger index.
    ///
    /// NOTE: The `prev_analytics` must be from `ledger_index - 1` or the results are invalid.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn update_ledger_size_analytics(
        &self,
        prev_analytics: &mut LedgerSizeAnalyticsResult,
        ledger_index: MilestoneIndex,
    ) -> Result<(), Error> {
        let (created, consumed) = tokio::try_join!(
            async {
                Result::<_, Error>::Ok(self.aggregate::<LedgerSizeAnalyticsResult>(
                        vec![
                            doc! { "$match": {
                                "metadata.booked.milestone_index": ledger_index,
                                "metadata.spent_metadata.spent.milestone_index": { "$ne": ledger_index }
                            } },
                            doc! { "$group" : {
                                "_id": null,
                                "total_key_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_key_bytes" } },
                                "total_data_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_data_bytes" } },
                                "total_storage_deposit_value": { "$sum": { "$toDecimal": { "$ifNull": [ "$output.storage_deposit_return_unlock_condition.amount", 0 ] } } }
                            } },
                            doc! { "$project": {
                                "total_storage_deposit_value": { "$toString": "$total_storage_deposit_value" },
                                "total_key_bytes": { "$toString": "$total_key_bytes" },
                                "total_data_bytes": { "$toString": "$total_data_bytes" },
                            } },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .unwrap_or_default())
            },
            async {
                Ok(self.aggregate::<LedgerSizeAnalyticsResult>(
                        vec![
                            doc! { "$match": {
                                "metadata.booked.milestone_index": { "$ne": ledger_index },
                                "metadata.spent_metadata.spent.milestone_index": ledger_index
                            } },
                            doc! { "$group" : {
                                "_id": null,
                                "total_key_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_key_bytes" } },
                                "total_data_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_data_bytes" } },
                                "total_storage_deposit_value": { "$sum": { "$toDecimal": { "$ifNull": [ "$output.storage_deposit_return_unlock_condition.amount", 0 ] } } }
                            } },
                            doc! { "$project": {
                                "total_storage_deposit_value": { "$toString": "$total_storage_deposit_value" },
                                "total_key_bytes": { "$toString": "$total_key_bytes" },
                                "total_data_bytes": { "$toString": "$total_data_bytes" },
                            } },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .unwrap_or_default())
            }
        )?;
        *prev_analytics += created;
        *prev_analytics -= consumed;

        Ok(())
    }
}
