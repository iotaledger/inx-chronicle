// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use crate::{db::MongoDb, types::stardust::block::MilestoneId};

/// Contains all informations regarding the treasury.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TreasuryDocument {
    pub milestone_id: MilestoneId,
    pub amount: u64,
}

impl TreasuryDocument {
    /// The treasury collection name.
    const COLLECTION: &'static str = "stardust_treasury";
}

/// Queries that are related to the treasury.
impl MongoDb {
    /// Inserts or updates treasury data.
    #[cfg(feature = "inx")]
    pub async fn upsert_treasury(&self, treasury_update: inx::TreasuryUpdate) -> Result<(), Error> {
        let treasury_document = TreasuryDocument {
            milestone_id: treasury_update.created.milestone_id.into(),
            amount: treasury_update.created.amount,
        };
        self.0
            .collection::<TreasuryDocument>(TreasuryDocument::COLLECTION)
            .insert_one(treasury_document, None)
            .await?;

        Ok(())
    }

    /// Returns the current state of the treasury.
    pub async fn get_treasury(&self) -> Result<Option<TreasuryDocument>, Error> {
        self.0
            .collection::<TreasuryDocument>(TreasuryDocument::COLLECTION)
            .find_one(doc! {}, None)
            .await
    }
}
