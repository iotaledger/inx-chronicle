// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::Stream;
use mongodb::{
    bson::{self, doc, Document},
    error::Error,
    options::{FindOptions, IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

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
#[derive(Copy, Clone, Debug)]
pub enum SortOrder {
    Newest,
    Oldest,
}

fn newest() -> Document {
    doc! { "at.milestone_index": -1, "output_id": -1, "is_spent": -1 }
}

fn oldest() -> Document {
    doc! { "at.milestone_index": 1, "output_id": 1, "is_spent": 1 }
}

/// Queries that are related to [`Output`](crate::types::stardust::block::Output)s.
impl MongoDb {
    /// Creates ledger update indexes.
    pub async fn create_ledger_update_indexes(&self) -> Result<(), Error> {
        let collection = self
            .0
            .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "address": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(false)
                            .name("address_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        collection
            .create_index(
                IndexModel::builder()
                    .keys(newest())
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .name("cursor_index".to_string())
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
    pub async fn insert_ledger_updates(
        &self,
        deltas: impl IntoIterator<Item = OutputWithMetadata>,
    ) -> Result<(), Error> {
        for delta in deltas {
            self.insert_output(delta.clone()).await?;
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
                self.0
                    .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                    .update_one(
                        doc! { "output_id": &doc.output_id, "is_spent": &doc.is_spent },
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

        self.0
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

        self.0
            .collection::<LedgerUpdateByMilestoneRecord>(LedgerUpdateDocument::COLLECTION)
            .find(
                doc! { "$and": queries },
                FindOptions::builder().limit(page_size as i64).sort(oldest()).build(),
            )
            .await
    }
}

mod analytics {
    use futures::TryStreamExt;
    use mongodb::bson;

    use super::*;
    use crate::types::stardust::milestone::MilestoneTimestamp;

    /// Address analytics result.

    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct AddressAnalyticsResult {
        /// The number of addresses used in the time period.
        pub total_addresses: u64,
        /// The number of addresses that received tokens in the time period.
        pub recv_addresses: u64,
        /// The number of addresses that sent tokens in the time period.
        pub send_addresses: u64,
    }

    impl MongoDb {
        /// Create aggregate statistics of all addresses.
        pub async fn get_address_analytics(
            &self,
            start_timestamp: MilestoneTimestamp,
            end_timestamp: MilestoneTimestamp,
        ) -> Result<Option<AddressAnalyticsResult>, Error> {
            Ok(self
                .0
                .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                .aggregate(
                    vec![
                        doc! { "$match": { "at.milestone_timestamp": { "$gt": start_timestamp, "$lt": end_timestamp } } },
                        doc! { "$facet": {
                            "total": [
                                { "$group" : {
                                    "_id": "$address",
                                    "transfers": { "$count": { } }
                                }},
                            ],
                            "recv": [
                                { "$match": { "is_spent": false } },
                                { "$group" : {
                                    "_id": "$address",
                                    "transfers": { "$count": { } }
                                }},
                            ],
                            "send": [
                                { "$match": { "is_spent": true } },
                                { "$group" : {
                                    "_id": "$address",
                                    "transfers": { "$count": { } }
                                }},
                            ],
                        } },
                        doc! { "$project": {
                            "total_addresses": { "$size": "$total.transfers" },
                            "recv_addresses": { "$size": "$recv.transfers" },
                            "send_addresses": { "$size": "$send.transfers" },
                        } },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .map(bson::from_document)
                .transpose()?)
        }
    }
}
