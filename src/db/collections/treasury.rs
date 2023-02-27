// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::doc,
    error::Error,
    options::{FindOneOptions, InsertManyOptions},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    db::{
        mongodb::{InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::stardust::payload::{
        milestone::{MilestoneId, MilestoneIndex},
        treasury_transaction::TreasuryTransactionPayload,
    },
};

/// Contains all information regarding the treasury.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TreasuryDocument {
    #[serde(rename = "_id")]
    milestone_index: MilestoneIndex,
    milestone_id: MilestoneId,
    amount: u64,
}

/// The stardust treasury collection.
pub struct TreasuryCollection {
    collection: mongodb::Collection<TreasuryDocument>,
}

impl MongoDbCollection for TreasuryCollection {
    const NAME: &'static str = "stardust_treasury";
    type Document = TreasuryDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

/// The latest treasury information.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TreasuryResult {
    pub milestone_id: MilestoneId,
    pub amount: u64,
}

/// Queries that are related to the treasury.
impl TreasuryCollection {
    /// Inserts treasury data.
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
        self.insert_one(treasury_document, None).await?;

        Ok(())
    }

    /// Inserts many treasury data.
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_treasury_payloads<I>(&self, payloads: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = (MilestoneIndex, MilestoneId, u64)>,
        I::IntoIter: Send + Sync,
    {
        let payloads = payloads
            .into_iter()
            .map(|(milestone_index, milestone_id, amount)| TreasuryDocument {
                milestone_index,
                milestone_id,
                amount,
            });
        self.insert_many_ignore_duplicates(payloads, InsertManyOptions::builder().ordered(false).build())
            .await?;

        Ok(())
    }

    /// Returns the current state of the treasury.
    pub async fn get_latest_treasury(&self) -> Result<Option<TreasuryResult>, Error> {
        self.find_one(doc! {}, FindOneOptions::builder().sort(doc! { "_id": -1 }).build())
            .await
    }
}
