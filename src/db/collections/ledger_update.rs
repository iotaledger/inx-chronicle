// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use futures::Stream;
use mongodb::{
    bson::{self, doc, Document},
    error::Error,
    options::{FindOptions, IndexOptions, InsertManyOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::instrument;

use crate::{
    db::MongoDb,
    types::{
        ledger::{MilestoneIndexTimestamp, OutputWithMetadata},
        stardust::block::{Address, OutputId},
        tangle::MilestoneIndex,
    },
};

/// Contains all information related to an output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct LedgerUpdateDocument {
    address: Address,
    output_id: OutputId,
    at: MilestoneIndexTimestamp,
    is_spent: bool,
}

impl LedgerUpdateDocument {
    /// The stardust outputs collection name.
    const COLLECTION: &'static str = "stardust_ledger_updates";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdateByAddressRecord {
    pub output_id: OutputId,
    pub at: MilestoneIndexTimestamp,
    pub is_spent: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdateByMilestoneRecord {
    pub address: Address,
    pub output_id: OutputId,
    pub is_spent: bool,
}

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SortOrder {
    Newest,
    Oldest,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Newest
    }
}

#[derive(Debug, Error)]
#[error("Invalid sort order descriptor. Expected `oldest` or `newest`, found `{0}`")]
#[allow(missing_docs)]
pub struct ParseSortError(String);

impl FromStr for SortOrder {
    type Err = ParseSortError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "oldest" => SortOrder::Oldest,
            "newest" => SortOrder::Newest,
            _ => Err(ParseSortError(s.to_string()))?,
        })
    }
}

fn newest() -> Document {
    doc! { "address": -1, "at.milestone_index": -1, "output_id": -1, "is_spent": -1 }
}

fn oldest() -> Document {
    doc! { "address": 1, "at.milestone_index": 1, "output_id": 1, "is_spent": 1 }
}

/// Queries that are related to [`Output`](crate::types::stardust::block::Output)s.
impl MongoDb {
    /// Creates ledger update indexes.
    pub async fn create_ledger_update_indexes(&self) -> Result<(), Error> {
        let collection = self
            .db
            .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION);

        collection
            .create_index(
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

    /// Removes all [`LedgerUpdateDocument`]s that are newer than a given [`MilestoneIndex`].
    #[instrument(name = "remove_ledger_updates_newer_than_milestone", skip_all, err, level = "trace")]
    pub async fn remove_ledger_updates_newer_than_milestone(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<usize, Error> {
        self.db
            .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
            .delete_many(doc! {"at.milestone_index": { "$gt": milestone_index }}, None)
            .await
            .map(|res| res.deleted_count as usize)
    }

    /// Inserts multiple ledger updates at once.
    #[instrument(name = "insert_ledger_updates", skip_all, err, level = "trace")]
    pub async fn insert_ledger_updates(
        &self,
        outputs: impl IntoIterator<Item = &OutputWithMetadata>,
    ) -> Result<(), Error> {
        let docs = outputs
            .into_iter()
            .filter_map(|output_with_metadata| {
                if let Some(&address) = output_with_metadata.output.owning_address() {
                    let at = output_with_metadata
                        .metadata
                        .spent_metadata
                        .map(|s| s.spent)
                        .unwrap_or(output_with_metadata.metadata.booked);

                    Some(LedgerUpdateDocument {
                        address,
                        output_id: output_with_metadata.metadata.output_id,
                        at,
                        is_spent: output_with_metadata.metadata.spent_metadata.is_some(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !docs.is_empty() {
            self.db
                .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                .insert_many(docs, InsertManyOptions::builder().ordered(false).build())
                .await?;
        }
        Ok(())
    }

    /// Upserts an [`Output`](crate::types::stardust::block::Output) together with its associated
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    #[instrument(skip_all, err, level = "trace")]
    pub async fn upsert_ledger_updates(
        &self,
        deltas: impl IntoIterator<Item = OutputWithMetadata>,
    ) -> Result<(), Error> {
        for delta in deltas {
            self.upsert_output(delta.clone()).await?;
            // Ledger updates
            if let Some(&address) = delta.output.owning_address() {
                let at = delta
                    .metadata
                    .spent_metadata
                    .map(|s| s.spent)
                    .unwrap_or(delta.metadata.booked);
                let doc = LedgerUpdateDocument {
                    address,
                    output_id: delta.metadata.output_id,
                    at,
                    is_spent: delta.metadata.spent_metadata.is_some(),
                };
                self.db
                    .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                    .update_one(
                        doc! { "address": &doc.address, "at.milestone_index": &doc.at.milestone_index, "output_id": &doc.output_id, "is_spent": &doc.is_spent },
                        doc! { "$setOnInsert": bson::to_document(&doc)? },
                        UpdateOptions::builder().upsert(true).build(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// Streams updates to the ledger for a given address.
    pub async fn stream_ledger_updates_by_address(
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
            let mut cursor_queries = vec![doc! { "at.milestone_index": { cmp1: milestone_index } }];
            if let Some((output_id, is_spent)) = rest {
                cursor_queries.push(doc! {
                    "at.milestone_index": milestone_index,
                    "output_id": { cmp1: output_id }
                });
                cursor_queries.push(doc! {
                    "at.milestone_index": milestone_index,
                    "output_id": output_id,
                    "is_spent": { cmp2: is_spent }
                });
            }
            queries.push(doc! { "$or": cursor_queries });
        }

        self.db
            .collection::<LedgerUpdateByAddressRecord>(LedgerUpdateDocument::COLLECTION)
            .find(
                doc! { "$and": queries },
                FindOptions::builder().limit(page_size as i64).sort(sort).build(),
            )
            .await
    }

    /// Streams updates to the ledger for a given milestone index (sorted by [`OutputId`]).
    pub async fn stream_ledger_updates_by_milestone(
        &self,
        milestone_index: MilestoneIndex,
        page_size: usize,
        cursor: Option<(OutputId, bool)>,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateByMilestoneRecord, Error>>, Error> {
        let (cmp1, cmp2) = ("$gt", "$gte");

        let mut queries = vec![doc! { "at.milestone_index": milestone_index }];

        if let Some((output_id, is_spent)) = cursor {
            let mut cursor_queries = vec![doc! { "output_id": { cmp1: output_id } }];
            cursor_queries.push(doc! {
                "output_id": output_id,
                "is_spent": { cmp2: is_spent }
            });
            queries.push(doc! { "$or": cursor_queries });
        }

        self.db
            .collection::<LedgerUpdateByMilestoneRecord>(LedgerUpdateDocument::COLLECTION)
            .find(
                doc! { "$and": queries },
                FindOptions::builder().limit(page_size as i64).sort(oldest()).build(),
            )
            .await
    }
}
