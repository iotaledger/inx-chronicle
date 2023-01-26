// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::Error;
use crate::{
    db::{collections::OutputCollection, MongoDbCollectionExt},
    types::tangle::MilestoneIndex,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AddressAnalyticsResult {
    pub address_with_balance_count: u64,
}

impl OutputCollection {
    /// Get ledger address analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_address_analytics(&self, ledger_index: MilestoneIndex) -> Result<AddressAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$group" : { "_id": "$details.address" } },
                    doc! { "$count" : "address_with_balance_count" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }
}
