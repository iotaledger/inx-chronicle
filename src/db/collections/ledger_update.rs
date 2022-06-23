// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::Stream;
use mongodb::{
    bson::{doc, Bson, Document},
    error::Error,
    options::{FindOptions, IndexOptions},
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
pub struct LedgerUpdatePerAddressRecord {
    pub output_id: OutputId,
    pub at: MilestoneIndexTimestamp,
    pub is_spent: bool,
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

fn index() -> Document {
    doc! { "address": 1, "at.milestone_index": -1, "output_id": 1 }
}

fn inverse_index() -> Document {
    doc! { "address": -1, "at.milestone_index": 1, "output_id": -1 }
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
                    .keys(index())
                    .options(
                        IndexOptions::builder()
                            .unique(false) // An output can be spent within the same milestone that it was created in.
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
        outputs_with_metadata: impl IntoIterator<Item = OutputWithMetadata>,
    ) -> Result<(), Error> {
        // TODO: Use `insert_many` and `update_many` to increase write performance.

        for OutputWithMetadata { output, metadata } in outputs_with_metadata {
            let at = metadata.spent.map_or(metadata.booked, |s| s.spent);
            let is_spent = metadata.spent.is_some();

            // Ledger updates
            for owner in output.owning_addresses() {
                let ledger_update_document = LedgerUpdateDocument {
                    address: owner,
                    output_id: metadata.output_id,
                    at,
                    is_spent,
                };

                // TODO: This is prone to overwriting and should be fixed in the future (GitHub issue: #218).
                let _ = self
                    .0
                    .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                    .insert_one(ledger_update_document, None)
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
        start_output_id: Option<OutputId>,
        order: SortOrder,
    ) -> Result<impl Stream<Item = Result<LedgerUpdatePerAddressRecord, Error>>, Error> {
        let mut filter = doc! {
            "address": { "$eq": address },
        };
        if let Some(milestone_index) = start_milestone_index {
            match order {
                SortOrder::Newest => {
                    filter.insert("at.milestone_index", doc! { "$lte": milestone_index });
                }
                SortOrder::Oldest => {
                    filter.insert("at.milestone_index", doc! { "$gte": milestone_index });
                }
            }
        }
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
            .sort(if order.is_newest() { inverse_index() } else { index() })
            .build();

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
            .sort(if order.is_newest() { inverse_index() } else { index() })
            .build();

        self.0
            .collection::<LedgerUpdatePerMilestoneRecord>(LedgerUpdateDocument::COLLECTION)
            .find(filter, options)
            .await
    }
}

#[cfg(feature = "analytics")]
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
