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
    types::tangle::MilestoneIndex,
};

/// The milestone's activity statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MilestoneActivityDocument {
    /// The [`MilestoneIndex`](MilestoneIndex) of the milestone.
    #[serde(rename = "_id")]
    milestone_index: MilestoneIndex,
    /// The milestone's past-cone activity.
    milestone_activity: MilestoneActivity,
}

/// The past-cone activity of a milestone.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MilestoneActivity {
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
pub struct MilestoneActivityCollection {
    collection: mongodb::Collection<MilestoneActivityDocument>,
}

impl MongoDbCollection for MilestoneActivityCollection {
    const NAME: &'static str = "stardust_milestone_activity";
    type Document = MilestoneActivityDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

impl MilestoneActivityCollection {
    /// Creates necessary indexes.
    pub async fn create_indexes(&self) -> Result<(), Error> {
        Ok(())
    }

    // /// Returns statistics for a given milestone.
    // pub async fn get_milestone_activity(&self, milestone_id: &MilestoneId) -> Result<Option<MilestoneStats>, Error> {
    //     self.aggregate(
    //         vec![
    //             doc! { "$match": { "_id": milestone_id } },
    //             doc! { "$replaceWith": "$milestone_activity" },
    //         ],
    //         None,
    //     )
    //     .await?
    //     .try_next()
    //     .await
    // }

    /// Returns the activity statistics for a range of milestones.
    pub async fn get_milestone_activity(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<MilestoneActivity, Error> {
        let queries = vec![
            doc! {
            "$nor": [
                { "metadata.referenced_by_milestone_index": { "$lt": start_index } },
                { "metadata.referenced_by_milestone_index": { "$gte": end_index } },
            ],
        }];
        Ok(self
            .aggregate(
                vec![doc! { 
                    "$match": { "$and": queries 
                } 
                }, 
                    doc! { "$count": "count" }],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }

    /// Inserts statistics associated with a milestone.
    #[instrument(skip(self, milestone_index), err, level = "trace")]
    pub async fn insert_milestone_activity(
        &self,
        milestone_index: MilestoneIndex,
        milestone_activity: MilestoneActivity,
    ) -> Result<(), Error> {
        let milestone_analytics_document = MilestoneActivityDocument {
            milestone_index,
            milestone_activity,
        };

        self.insert_one(milestone_analytics_document, None).await?;

        Ok(())
    }
}
