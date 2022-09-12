// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    db::{
        mongodb::{MongoCollectionExt, MongoDbCollection},
        MongoDb,
    },
    types::stardust::block::payload::MilestoneId,
};

/// The milestone's stats.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MilestoneAnalyticsDocument {
    /// The [`MilestoneId`](MilestoneId) of the milestone.
    #[serde(rename = "_id")]
    milestone_id: MilestoneId,
    /// The milestone's past-cone stats.
    milestone_stats: MilestoneStats,
}

/// The past-cone stats for a milestone.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MilestoneStats {
    /// The number of blocks referenced by a milestone.
    pub num_blocks: u32,
    /// The number of blocks referenced by a milestone that contain a payload.
    pub num_tx_payload: u32,
    /// The number of blocks containing a treasury transaction payload.
    pub num_treasury_tx_payload: u32,
    /// The number of blocks containing a milestone payload.
    pub num_milestone_payload: u32,
    /// The number of blocks containing a tagged data payload.
    pub num_tagged_data_payload: u32,
    /// The number of blocks referenced by a milestone that contain no payload.
    pub num_no_payload: u32,
    /// The number of blocks containing a confirmed transaction.
    pub num_confirmed_tx: u32,
    /// The number of blocks containing a conflicting transaction.
    pub num_conflicting_tx: u32,
    /// The number of blocks containing no transaction.
    pub num_no_tx: u32,
}

/// The Stardust milestone analytics collection.
pub struct MilestoneAnalyticsCollection {
    collection: mongodb::Collection<MilestoneAnalyticsDocument>,
}

impl MongoDbCollection for MilestoneAnalyticsCollection {
    const NAME: &'static str = "stardust_milestone_analytics";
    type Document = MilestoneAnalyticsDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

impl MilestoneAnalyticsCollection {
    /// Creates necessary indexes.
    pub async fn create_indexes(&self) -> Result<(), Error> {
        Ok(())
    }

    /// Returns statistics for a given milestone.
    pub async fn get_milestone_stats(&self, milestone_id: &MilestoneId) -> Result<Option<MilestoneStats>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": { "_id": milestone_id } },
                doc! { "$replaceWith": "$milestone_stats" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Inserts statistics associated with a milestone.
    #[instrument(skip(self, milestone_id), err, level = "trace")]
    pub async fn insert_milestone_stats(
        &self,
        milestone_id: MilestoneId,
        milestone_stats: MilestoneStats,
    ) -> Result<(), Error> {
        let milestone_analytics_document = MilestoneAnalyticsDocument {
            milestone_id,
            milestone_stats,
        };

        self.insert_one(milestone_analytics_document, None).await?;

        Ok(())
    }
}
