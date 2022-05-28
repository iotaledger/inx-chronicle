// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::Stream;
use mongodb::{
    bson::{doc, to_bson, Bson},
    error::Error,
    options::FindOptions,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
struct LedgerUpdateDocument {
    owner: Address,
    output_id: OutputId,
    at: MilestoneIndexTimestamp,
    is_spent: bool,
}

impl LedgerUpdateDocument {
    /// The stardust outputs collection name.
    const COLLECTION: &'static str = "stardust_ledger_updates";
}

#[derive(Copy, Clone, Debug)]
pub enum SortOrder {
    Newest,
    Oldest,
}

impl From<SortOrder> for Bson {
    fn from(value: SortOrder) -> Self {
        match value {
            SortOrder::Newest => Bson::Int32(1),
            SortOrder::Oldest => Bson::Int32(-1),
        }
    }
}

// TODO find better name
/// Contains all informations related to an output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerUpdateRecord {
    pub output_id: OutputId,
    pub at: MilestoneIndexTimestamp,
    pub is_spent: bool,
}

/// Queries that are related to [`Output`](crate::types::stardust::block::Output)s.
impl MongoDb {
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
                    owner,
                    output_id: metadata.output_id.clone(),
                    at: metadata.spent.clone().map_or(metadata.booked.clone(), |s| s.spent),
                    is_spent: metadata.spent.is_some(),
                };

                self.0
                    .collection::<LedgerUpdateDocument>(LedgerUpdateDocument::COLLECTION)
                    .insert_one(ledger_update_document, None)
                    .await?;
            }

            // Upsert outputs
            self.upsert_output_with_metadata(metadata.output_id.clone(), output, metadata)
                .await?;
        }

        Ok(())
    }

    /// Get updates to the ledger for a given address.
    pub async fn get_ledger_updates(
        &self,
        address: Address,
        page_size: Option<i64>,
        cursor: (MilestoneIndex, OutputId),
        order: SortOrder,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateRecord, Error>>, Error> {
        let mut options = FindOptions::default();
        options.limit = page_size;
        options.sort = Some(doc! {"at.milestone_index": order, "output_id": order});

        self.0
            .collection::<LedgerUpdateRecord>(LedgerUpdateDocument::COLLECTION)
            .find(
                doc! {
                    "address": { "$eq": to_bson(&address)? },
                    "at.milestone_index": { "$gte": to_bson(&cursor.0)? },
                    "output_id": { "$gte": to_bson(&cursor.1)? },
                },
                options,
            )
            .await
    }
}
