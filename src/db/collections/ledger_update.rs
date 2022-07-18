// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::Stream;
use mongodb::{
    bson::{self, doc, Bson, Document},
    error::Error,
    options::{FindOptions, IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        ledger::{MilestoneIndexTimestamp, OutputWithMetadata},
        stardust::block::{Address, OutputAmount, OutputId},
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
    cursor: String,
}

impl LedgerUpdateDocument {
    /// The stardust outputs collection name.
    const COLLECTION: &'static str = "stardust_ledger_updates";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdatePerAddressRecord {
    pub output_id: OutputId,
    pub at: MilestoneIndexTimestamp,
    pub is_spent: bool,
    pub is_trivial_unlock: bool,
    pub amount: OutputAmount,
    pub cursor: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdatePerMilestoneRecord {
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

impl SortOrder {
    fn is_newest(&self) -> bool {
        matches!(self, SortOrder::Newest)
    }

    #[allow(dead_code)]
    fn is_oldest(&self) -> bool {
        matches!(self, SortOrder::Oldest)
    }
}

impl From<SortOrder> for Bson {
    fn from(value: SortOrder) -> Self {
        match value {
            SortOrder::Newest => Bson::Int32(-1),
            SortOrder::Oldest => Bson::Int32(1),
        }
    }
}

fn newest() -> Document {
    doc! { "address": 1, "cursor": -1 }
}

fn oldest() -> Document {
    doc! { "address": -1, "cursor": 1 }
}

/// Queries that are related to [`Output`](crate::types::stardust::block::Output)s.
impl MongoDb {
    /// Creates ledger update indexes.
    pub async fn create_ledger_update_indexes(&self) -> Result<(), Error> {
        let collection = self
            .0
            .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION);

        // TODO: Check if this index is even being used.
        collection
            .create_index(
                IndexModel::builder()
                    .keys(newest())
                    .options(
                        IndexOptions::builder()
                            // An output can be spent within the same milestone that it was created in.
                            .unique(true)
                            .name("ledger_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }

    /// Upserts a [`Output`](crate::types::stardust::block::Output) together with its associated
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    pub async fn insert_ledger_updates(
        &self,
        deltas: impl IntoIterator<Item = OutputWithMetadata>,
    ) -> Result<(), Error> {
        for delta in deltas {
            self.insert_output(delta.clone()).await?;
            // Ledger updates
            if let Some(&address) = delta.output.owning_address() {
                let at = delta.metadata.spent.map(|s| s.spent).unwrap_or(delta.metadata.booked);
                let doc = LedgerUpdateDocument {
                    address,
                    output_id: delta.metadata.output_id,
                    at,
                    is_spent: delta.metadata.spent.is_some(),
                    cursor: format!(
                        "{}.{}.{}",
                        at.milestone_index,
                        delta.metadata.output_id.to_hex(),
                        delta.metadata.spent.is_some()
                    ),
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
    pub async fn stream_ledger_updates_for_address(
        &self,
        address: &Address,
        page_size: usize,
        start_milestone_index: Option<MilestoneIndex>,
        start_output_id_is_spent: Option<(OutputId, bool)>,
        order: SortOrder,
    ) -> Result<impl Stream<Item = Result<LedgerUpdatePerAddressRecord, Error>>, Error> {
        let cursor = match (start_milestone_index, start_output_id_is_spent) {
            (Some(milestone_index), Some((output_id, is_spent))) => {
                Some(format!("{}.{}.{}", milestone_index, output_id.to_hex(), is_spent))
            }
            (Some(milestone_index), None) => Some(milestone_index.to_string()),
            _ => None,
        };

        let (sort, cmp) = match order {
            SortOrder::Newest => (newest(), "$gte"),
            SortOrder::Oldest => (oldest(), "$lte"),
        };

        let filter = if let Some(c) = cursor {
            doc! { "address": address, "cursor": { cmp: c } }
        } else {
            doc! {}
        };

        let options = FindOptions::builder().limit(page_size as i64).sort(sort).build();

        self.0
            .collection::<LedgerUpdatePerAddressRecord>(LedgerUpdateDocument::COLLECTION)
            .find(filter, options)
            .await
    }

    /// Streams updates to the ledger for a given milestone index.
    pub async fn stream_ledger_updates_for_index(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<LedgerUpdatePerMilestoneRecord, Error>>, Error> {
        self.0
            .collection::<LedgerUpdatePerMilestoneRecord>(LedgerUpdateDocument::COLLECTION)
            .find(
                doc! {
                    "at.milestone_index": { "$eq": milestone_index },
                },
                None,
            )
            .await
    }

    /// Streams updates to the ledger for a given milestone index (sorted by [`OutputId`]).
    pub async fn stream_ledger_updates_for_index_paginated(
        &self,
        milestone_index: MilestoneIndex,
        page_size: usize,
        start_output_id: Option<OutputId>,
        order: SortOrder,
    ) -> Result<impl Stream<Item = Result<LedgerUpdatePerMilestoneRecord, Error>>, Error> {
        let mut filter = doc! {
            "at.milestone_index": { "$eq": milestone_index }
        };
        if let Some(output_id) = start_output_id {
            match order {
                SortOrder::Newest => {
                    filter.insert("output_id", doc! { "$lte": output_id });
                }
                SortOrder::Oldest => {
                    filter.insert("output_id", doc! { "$gte": output_id });
                }
            }
        }

        let options = FindOptions::builder()
            .limit(page_size as i64)
            .sort(if order.is_newest() { newest() } else { oldest() })
            .build();

        self.0
            .collection::<LedgerUpdatePerMilestoneRecord>(LedgerUpdateDocument::COLLECTION)
            .find(filter, options)
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
