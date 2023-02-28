// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::{doc, Document},
    error::Error,
    options::{FindOptions, IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::SortOrder;
use crate::{
    db::{
        mongodb::{InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::{
        ledger::{LedgerOutput, LedgerSpent},
        output::OutputId,
        payload::milestone::{MilestoneIndex, MilestoneIndexTimestamp, MilestoneTimestamp},
        Address,
    },
};

/// The [`Id`] of a [`LedgerUpdateDocument`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Id {
    milestone_index: MilestoneIndex,
    output_id: OutputId,
    is_spent: bool,
}

/// Contains all information related to an output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LedgerUpdateDocument {
    _id: Id,
    address: Address,
    milestone_timestamp: MilestoneTimestamp,
}

/// The stardust ledger updates collection.
pub struct LedgerUpdateCollection {
    collection: mongodb::Collection<LedgerUpdateDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for LedgerUpdateCollection {
    const NAME: &'static str = "stardust_ledger_updates";
    type Document = LedgerUpdateDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), Error> {
        self.create_index(
            IndexModel::builder()
                .keys(newest())
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("ledger_update_index".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdateByAddressRecord {
    pub at: MilestoneIndexTimestamp,
    pub output_id: OutputId,
    pub is_spent: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdateByMilestoneRecord {
    pub address: Address,
    pub output_id: OutputId,
    pub is_spent: bool,
}

fn newest() -> Document {
    doc! { "address": -1, "_id.milestone_index": -1, "_id.output_id": -1, "_id.is_spent": -1 }
}

fn oldest() -> Document {
    doc! { "address": 1, "_id.milestone_index": 1, "_id.output_id": 1, "_id.is_spent": 1 }
}

/// Queries that are related to [`Output`](crate::model::Output)s.
impl LedgerUpdateCollection {
    /// Inserts [`LedgerSpent`] updates.
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_spent_ledger_updates<'a, I>(&self, outputs: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = &'a LedgerSpent>,
        I::IntoIter: Send + Sync,
    {
        let ledger_updates = outputs.into_iter().filter_map(
            |LedgerSpent {
                 output: LedgerOutput { output_id, output, .. },
                 spent_metadata,
             }| {
                // Ledger updates
                output.owning_address().map(|&address| LedgerUpdateDocument {
                    _id: Id {
                        milestone_index: spent_metadata.spent.milestone_index,
                        output_id: *output_id,
                        is_spent: true,
                    },
                    address,
                    milestone_timestamp: spent_metadata.spent.milestone_timestamp,
                })
            },
        );
        self.insert_many_ignore_duplicates(ledger_updates, InsertManyOptions::builder().ordered(false).build())
            .await?;

        Ok(())
    }

    /// Inserts unspent [`LedgerOutput`] updates.
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_unspent_ledger_updates<'a, I>(&self, outputs: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = &'a LedgerOutput>,
        I::IntoIter: Send + Sync,
    {
        let ledger_updates = outputs.into_iter().filter_map(
            |LedgerOutput {
                 output_id,
                 booked,
                 output,
                 ..
             }| {
                // Ledger updates
                output.owning_address().map(|&address| LedgerUpdateDocument {
                    _id: Id {
                        milestone_index: booked.milestone_index,
                        output_id: *output_id,
                        is_spent: false,
                    },
                    address,
                    milestone_timestamp: booked.milestone_timestamp,
                })
            },
        );
        self.insert_many_ignore_duplicates(ledger_updates, InsertManyOptions::builder().ordered(false).build())
            .await?;

        Ok(())
    }

    /// Streams updates to the ledger for a given address.
    pub async fn get_ledger_updates_by_address(
        &self,
        address: &Address,
        page_size: usize,
        cursor: Option<(MilestoneIndex, Option<(OutputId, bool)>)>,
        order: SortOrder,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateByAddressRecord, Error>>, Error> {
        let (sort, cmp1, cmp2) = match order {
            SortOrder::Newest => (newest(), "$lt", "$lte"),
            SortOrder::Oldest => (oldest(), "$gt", "$gte"),
        };

        let mut queries = vec![doc! { "address": address }];

        if let Some((milestone_index, rest)) = cursor {
            let mut cursor_queries = vec![doc! { "_id.milestone_index": { cmp1: milestone_index } }];
            if let Some((output_id, is_spent)) = rest {
                cursor_queries.push(doc! {
                    "_id.milestone_index": milestone_index,
                    "_id.output_id": { cmp1: output_id }
                });
                cursor_queries.push(doc! {
                    "_id.milestone_index": milestone_index,
                    "_id.output_id": output_id,
                    "_id.is_spent": { cmp2: is_spent }
                });
            }
            queries.push(doc! { "$or": cursor_queries });
        }

        Ok(self
            .find::<LedgerUpdateDocument>(
                doc! { "$and": queries },
                FindOptions::builder().limit(page_size as i64).sort(sort).build(),
            )
            .await?
            .map_ok(|doc| LedgerUpdateByAddressRecord {
                at: doc._id.milestone_index.with_timestamp(doc.milestone_timestamp),
                output_id: doc._id.output_id,
                is_spent: doc._id.is_spent,
            }))
    }

    /// Streams updates to the ledger for a given milestone index (sorted by [`OutputId`]).
    pub async fn get_ledger_updates_by_milestone(
        &self,
        milestone_index: MilestoneIndex,
        page_size: usize,
        cursor: Option<(OutputId, bool)>,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateByMilestoneRecord, Error>>, Error> {
        let (cmp1, cmp2) = ("$gt", "$gte");

        let mut queries = vec![doc! { "_id.milestone_index": milestone_index }];

        if let Some((output_id, is_spent)) = cursor {
            let mut cursor_queries = vec![doc! { "_id.output_id": { cmp1: output_id } }];
            cursor_queries.push(doc! {
                "_id.output_id": output_id,
                "_id.is_spent": { cmp2: is_spent }
            });
            queries.push(doc! { "$or": cursor_queries });
        }

        Ok(self
            .find::<LedgerUpdateDocument>(
                doc! { "$and": queries },
                FindOptions::builder().limit(page_size as i64).sort(oldest()).build(),
            )
            .await?
            .map_ok(|doc| LedgerUpdateByMilestoneRecord {
                address: doc.address,
                output_id: doc._id.output_id,
                is_spent: doc._id.is_spent,
            }))
    }
}
