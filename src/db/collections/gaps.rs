// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, ops::RangeInclusive};

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::doc,
    error::Error,
    options::{FindOptions, IndexOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{db::MongoDb, types::tangle::MilestoneIndex};

// TODO: Make sure updating the config doesn't break this

/// Chronicle gap record.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct GapsDocument {
    start: MilestoneIndex,
    end: MilestoneIndex,
}

impl GapsDocument {
    /// The stardust gaps collection name.
    const COLLECTION: &'static str = "stardust_gaps";
}

lazy_static::lazy_static! {
    static ref GAPS: RwLock<Option<HashMap<MilestoneIndex, MilestoneIndex>>> = Default::default();
}

impl MongoDb {
    /// Creates gap indexes.
    pub async fn create_gap_indexes(&self) -> Result<(), Error> {
        let collection = self.0.collection::<GapsDocument>(GapsDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "start": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .name("gap_start_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "end": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .name("gap_end_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }

    /// This is a kind of trashy workaround, but here are the basics:
    /// 1. Since transactions are hard, we're going to just use a RwLock to sync.
    /// 2. The actual data is stored in memory (read from the db to initialize).
    /// 3. The in-memory data is modified instead of db data, then the collection is
    ///     cleared and replaced with the in-memory data before unlocking. This
    ///     effectively makes the operation an atomic transaction.
    ///
    /// This is kind of fine because this table is very small, and only stored in
    /// the db because we need to persist it when the app closes.
    async fn modify_gaps<F>(&self, f: F) -> Result<(), Error>
    where
        F: Fn(&mut HashMap<MilestoneIndex, MilestoneIndex>),
    {
        let mut gaps = GAPS.write().await;
        if gaps.is_none() {
            let map = self
                .0
                .collection::<GapsDocument>(GapsDocument::COLLECTION)
                .find(doc! {}, None)
                .await?
                .map_ok(|d| (d.start, d.end))
                .try_collect::<HashMap<_, _>>()
                .await?;
            *gaps = Some(map);
        }
        let map = gaps.as_mut().unwrap();
        f(map);
        self.0
            .collection::<GapsDocument>(GapsDocument::COLLECTION)
            .delete_many(doc! {}, None)
            .await?;
        if !map.is_empty() {
            self.0
                .collection::<GapsDocument>(GapsDocument::COLLECTION)
                .insert_many(
                    map.iter().map(|(start, end)| GapsDocument {
                        start: *start,
                        end: *end,
                    }),
                    None,
                )
                .await?;
        }

        Ok(())
    }

    /// Marks a milestone as synced.
    pub async fn complete_milestone(&self, index: MilestoneIndex) -> Result<(), Error> {
        // First get the gap containing this index, if there is one.
        let gap = self
            .0
            .collection::<GapsDocument>(GapsDocument::COLLECTION)
            .find_one(
                doc! {
                    "start": { "$lte": index },
                    "end": { "$gte": index },
                },
                None,
            )
            .await?;

        if let Some(GapsDocument { start, end }) = gap {
            self.modify_gaps(|map| {
                // Delete that gap.
                map.remove(&start);
                // Split the gap into two.
                let (rec1, rec2) = (
                    GapsDocument { start, end: index - 1 },
                    GapsDocument { start: index + 1, end },
                );
                // If either contains any indexes, insert them.
                if rec1.start <= rec1.end {
                    map.insert(rec1.start, rec1.end);
                }
                if rec2.start <= rec2.end {
                    map.insert(rec2.start, rec2.end);
                }
            })
            .await?;
        }
        Ok(())
    }

    /// Inserts a gap into the gaps collection.
    pub async fn insert_gap(&self, start: MilestoneIndex, end: MilestoneIndex) -> Result<(), Error> {
        self.modify_gaps(|map| {
            map.insert(start, end);
        })
        .await?;
        Ok(())
    }

    /// Retrieves gaps in the synced milestones.
    pub async fn get_gaps(&self) -> Result<impl Stream<Item = Result<RangeInclusive<MilestoneIndex>, Error>>, Error> {
        Ok(self
            .0
            .collection::<GapsDocument>(GapsDocument::COLLECTION)
            .find(doc! {}, FindOptions::builder().sort(doc! { "start": 1 }).build())
            .await?
            .map_ok(|d| d.start..=d.end))
    }
}
