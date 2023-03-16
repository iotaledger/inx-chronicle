// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::db::{
    mongodb::collections::{BlockCollection, OutputCollection, ParentsCollection},
    MongoDb, MongoDbCollection, MongoDbCollectionExt,
};
use mongodb::{bson::{doc, Document}, options::CreateCollectionOptions};

use super::Migration;

pub struct Migrate;

#[async_trait]
impl Migration for Migrate {
    const ID: usize = 1;
    const APP_VERSION: &'static str = "1.0.0-beta.37";
    const DATE: time::Date = time::macros::date!(2023 - 03 - 14);

    async fn migrate(db: &MongoDb) -> eyre::Result<()> {
        let collection = db.collection::<OutputCollection>();

        collection.drop_index("output_address_unlock_index", None).await?;
        collection
            .drop_index("output_storage_deposit_return_unlock_index", None)
            .await?;
        collection.drop_index("output_timelock_unlock_index", None).await?;
        collection.drop_index("output_expiration_unlock_index", None).await?;
        collection
            .drop_index("output_state_controller_unlock_index", None)
            .await?;
        collection
            .drop_index("output_governor_address_unlock_index", None)
            .await?;
        collection
            .drop_index("output_immutable_alias_address_unlock_index", None)
            .await?;
        collection.drop_index("block_parents_index", None).await?;

        let options = CreateCollectionOptions::builder()
            .capped(true)
            .max(100000) // Maximum number of documents
            .build();
        db.create_indexes_with_options::<ParentsCollection>(options).await?;

        // FIXME: oh no, how do we get the referenced indexes of the parents??
        let _ = db
            .collection::<BlockCollection>()
            .aggregate::<Document>(
                [
                    doc! { "$unwind": "$block.parents" },
                    doc! { "$project": {
                        "_id": 0,
                        "parent_id": "$block.parents",
                        "parent_referenced_index": 42,
                        "child_id": "$_id",
                        "child_referenced_index": "$metadata.referenced_by_milestone_index",
                    } },
                    doc! { "$merge": {
                        "into": ParentsCollection::NAME,
                        "on": [ "parent_id", "parent_referenced_index", "child_id", "child_referenced_index" ],
                        "whenMatched": "keepExisting",
                    }},
                ],
                None,
            )
            .await?;

        Ok(())
    }
}
