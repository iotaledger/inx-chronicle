// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::doc,
    error::Error,
    options::{FindOneOptions, IndexOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        stardust::block::{MilestoneId, TreasuryTransactionPayload},
        tangle::MilestoneIndex,
    },
};

/// Contains all information regarding the treasury.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct TreasuryDocument {
    milestone_index: MilestoneIndex,
    milestone_id: MilestoneId,
    amount: u64,
}

impl TreasuryDocument {
    /// The treasury collection name.
    const COLLECTION: &'static str = "stardust_treasury";
}

/// The latest treasury information.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TreasuryRecord {
    pub milestone_id: MilestoneId,
    pub amount: u64,
}

/// Queries that are related to the treasury.
impl MongoDb {
    /// Creates ledger update indexes.
    pub async fn create_treasury_indexes(&self) -> Result<(), Error> {
        let collection = self.0.collection::<TreasuryDocument>(TreasuryDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "at.milestone_index": -1, "milestone_id": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .name("treasury_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }

    /// Inserts or updates treasury data.
    pub async fn insert_treasury(
        &self,
        milestone_index: MilestoneIndex,
        payload: &TreasuryTransactionPayload,
    ) -> Result<(), Error> {
        let treasury_document = TreasuryDocument {
            milestone_index,
            milestone_id: payload.input_milestone_id,
            amount: payload.output_amount,
        };
        self.0
            .collection::<TreasuryDocument>(TreasuryDocument::COLLECTION)
            .insert_one(treasury_document, None)
            .await?;

        Ok(())
    }

    /// Returns the current state of the treasury.
    pub async fn get_latest_treasury(&self) -> Result<Option<TreasuryRecord>, Error> {
        self.0
            .collection::<TreasuryRecord>(TreasuryDocument::COLLECTION)
            .find_one(
                doc! {},
                FindOneOptions::builder().sort(doc! { "milestone_index": -1 }).build(),
            )
            .await
    }
}
