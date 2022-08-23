// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use futures::Stream;
use mongodb::{
    bson::{doc, Document},
    error::Error,
    options::{FindOptions, IndexOptions, InsertManyOptions},
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

    /// Upserts an [`Output`](crate::types::stardust::block::Output) together with its associated
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_ledger_updates(&self, outputs: impl Iterator<Item = &OutputWithMetadata>) -> Result<(), Error> {
        let ledger_updates = outputs
            .filter_map(|o| {
                // Ledger updates
                if let Some(&address) = o.output.owning_address() {
                    Some(LedgerUpdateDocument {
                        address,
                        output_id: o.metadata.output_id,
                        at: o.metadata.spent_metadata.map(|s| s.spent).unwrap_or(o.metadata.booked),
                        is_spent: o.metadata.spent_metadata.is_some(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for batch in ledger_updates.chunks(10000) {
            self.collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                .insert_many_ignore_duplicates(batch, InsertManyOptions::builder().ordered(false).build())
                .await?;
        }

        Ok(())
    }

    /// Upserts an [`Output`](crate::types::stardust::block::Output) together with its associated
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_ledger_update(&self, output: &OutputWithMetadata) -> Result<(), Error> {
        if let Some(&address) = output.output.owning_address() {
            let doc = LedgerUpdateDocument {
                address,
                output_id: output.metadata.output_id,
                at: output
                    .metadata
                    .spent_metadata
                    .map(|s| s.spent)
                    .unwrap_or(output.metadata.booked),
                is_spent: output.metadata.spent_metadata.is_some(),
            };
            self.db
                .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                .insert_one(doc, None)
                .await?;
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
