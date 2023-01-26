// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use decimal::d128;
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::Error;
use crate::{
    db::{collections::OutputCollection, MongoDbCollectionExt},
    types::tangle::MilestoneIndex,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UnlockConditionAnalyticsResult {
    pub timelock_count: u64,
    pub timelock_value: u64,
    pub expiration_count: u64,
    pub expiration_value: u64,
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_value: u64,
    pub storage_deposit_return_inner_value: u64,
}

impl OutputCollection {
    /// Gets analytics about unlock conditions.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_unlock_condition_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<UnlockConditionAnalyticsResult, Error> {
        #[derive(Default, Deserialize)]
        struct Res {
            count: u64,
            value: d128,
        }

        let query = |kind: &'static str| async move {
            Result::<Res, Error>::Ok(
                self.aggregate(
                    vec![
                        doc! { "$match": {
                            format!("output.{kind}"): { "$exists": true },
                            "metadata.booked.milestone_index": { "$lte": ledger_index },
                            "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                        } },
                        doc! { "$group": {
                            "_id": null,
                            "count": { "$sum": 1 },
                            "value": { "$sum": { "$toDecimal": "$output.amount" } },
                        } },
                        doc! { "$project": {
                            "count": 1,
                            "value": { "$toString": "$value" },
                        } },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .unwrap_or_default(),
            )
        };

        #[derive(Default, Deserialize)]
        struct ResSdruc {
            count: u64,
            value: d128,
            inner: d128,
        }

        let sdruc_query = async move {
            Result::<ResSdruc, Error>::Ok(
            self.aggregate(
                vec![
                    doc! { "$match": {
                        "output.storage_deposit_return_unlock_condition": { "$exists": true },
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$group": {
                        "_id": null,
                        "count": { "$sum": 1 },
                        "value": { "$sum": { "$toDecimal": "$output.amount" } },
                        "inner": { "$sum": { "$toDecimal": "$output.storage_deposit_return_unlock_condition.amount" } },
                    } },
                    doc! { "$project": {
                        "count": 1,
                        "value": { "$toString": "$value" },
                        "inner": { "$toString": "$inner" },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
        };

        let (timelock, expiration, sdruc) = tokio::try_join!(
            query("timelock_unlock_condition"),
            query("expiration_unlock_condition"),
            sdruc_query,
        )?;

        Ok(UnlockConditionAnalyticsResult {
            timelock_count: timelock.count,
            timelock_value: timelock.value.to_string().parse().unwrap(),
            expiration_count: expiration.count,
            expiration_value: expiration.value.to_string().parse().unwrap(),
            storage_deposit_return_count: sdruc.count,
            storage_deposit_return_value: sdruc.value.to_string().parse().unwrap(),
            storage_deposit_return_inner_value: sdruc.inner.to_string().parse().unwrap(),
        })
    }
}
