// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::Stream;
use mongodb::{
    bson::{doc, Bson},
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

/// Contains all informations related to an output.
#[allow(missing_docs)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerUpdateDocument {
    pub address: Address,
    pub output_id: OutputId,
    pub at: MilestoneIndexTimestamp,
    pub is_spent: bool,
}

impl LedgerUpdateDocument {
    /// The stardust outputs collection name.
    const COLLECTION: &'static str = "stardust_ledger_updates";
}

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug)]
pub enum SortOrder {
    Newest,
    Oldest,
}

impl From<SortOrder> for Bson {
    fn from(value: SortOrder) -> Self {
        match value {
            SortOrder::Newest => Bson::Int32(-1),
            SortOrder::Oldest => Bson::Int32(1),
        }
    }
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
                    .keys(doc! { "address": 1, "at.milestone_index": -1, "output_id": 1 })
                    .options(
                        IndexOptions::builder()
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
        outputs_with_metadata: impl IntoIterator<Item = OutputWithMetadata>,
    ) -> Result<(), Error> {
        // TODO: Use `insert_many` and `update_many` to increase write performance.

        for OutputWithMetadata { output, metadata } in outputs_with_metadata {
            // Ledger updates
            for owner in output.owning_addresses() {
                let ledger_update_document = LedgerUpdateDocument {
                    address: owner,
                    output_id: metadata.output_id,
                    at: metadata.spent.map_or(metadata.booked, |s| s.spent),
                    is_spent: metadata.spent.is_some(),
                };

                // TODO: This is prone to overwriting and should be fixed in the future (GitHub issue: #218).
                self.0
                    .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                    .insert_one(ledger_update_document, None)
                    .await?;
            }
        }

        Ok(())
    }

    /// Get updates to the ledger for a given address.
    pub async fn get_ledger_updates(
        &self,
        address: &Address,
        page_size: usize,
        start_milestone_index: Option<MilestoneIndex>,
        start_output_id: Option<OutputId>,
        order: SortOrder,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateDocument, Error>>, Error> {
        let options = FindOptions::builder()
            .limit(page_size as i64)
            .sort(doc! {"at.milestone_index": order, "output_id": order})
            .build();

        let mut doc = doc! {
            "address": { "$eq": address },
        };
        if let Some(milestone_index) = start_milestone_index {
            match order {
                SortOrder::Newest => {
                    doc.insert("at.milestone_index", doc! { "$lte": milestone_index });
                }
                SortOrder::Oldest => {
                    doc.insert("at.milestone_index", doc! { "$gte": milestone_index });
                }
            }
        }
        if let Some(output_id) = start_output_id {
            match order {
                SortOrder::Newest => {
                    doc.insert("output_id", doc! { "$lte": output_id });
                }
                SortOrder::Oldest => {
                    doc.insert("output_id", doc! { "$gte": output_id });
                }
            }
        }

        self.0
            .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
            .find(doc, options)
            .await
    }
}
