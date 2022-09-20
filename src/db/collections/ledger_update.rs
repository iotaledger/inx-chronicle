// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::{doc, Document},
    error::Error,
    options::{FindOptions, IndexOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use super::SortOrder;
use crate::{
    db::{
        mongodb::{MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    types::{
        ledger::MilestoneIndexTimestamp,
        stardust::{
            block::{output::OutputId, Address},
            milestone::MilestoneTimestamp,
        },
        tangle::MilestoneIndex,
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

impl MongoDbCollection for LedgerUpdateCollection {
    const NAME: &'static str = "stardust_ledger_updates";
    type Document = LedgerUpdateDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdateByAddressRecord {
    pub at: MilestoneIndexTimestamp,
    pub output_id: OutputId,
    pub is_spent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

/// Queries that are related to [`Output`](crate::types::stardust::block::Output)s.
impl LedgerUpdateCollection {
    /// Creates ledger update indexes.
    pub async fn create_indexes(&self) -> Result<(), Error> {
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
                at: MilestoneIndexTimestamp {
                    milestone_index: doc._id.milestone_index,
                    milestone_timestamp: doc.milestone_timestamp,
                },
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
