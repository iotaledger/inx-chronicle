// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::doc,
    error::Error,
    options::{FindOneOptions, InsertManyOptions},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::INSERT_BATCH_SIZE;
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
    #[serde(rename = "_id")]
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
pub struct TreasuryResult {
    pub milestone_id: MilestoneId,
    pub amount: u64,
}

/// Queries that are related to the treasury.
impl MongoDb {
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
        self.db
            .collection::<TreasuryDocument>(TreasuryDocument::COLLECTION)
            .insert_one(treasury_document, None)
            .await?;

        Ok(())
    }

    /// Inserts many treasury data.
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_treasury_payloads(
        &self,
        payloads: impl IntoIterator<Item = (MilestoneIndex, &TreasuryTransactionPayload)>,
    ) -> Result<(), Error> {
        let payloads = payloads
            .into_iter()
            .map(|(milestone_index, payload)| TreasuryDocument {
                milestone_index,
                milestone_id: payload.input_milestone_id,
                amount: payload.output_amount,
            })
            .collect::<Vec<_>>();
        for batch in payloads.chunks(INSERT_BATCH_SIZE) {
            self.collection::<TreasuryDocument>(TreasuryDocument::COLLECTION)
                .insert_many_ignore_duplicates(batch, InsertManyOptions::builder().ordered(false).build())
                .await?;
        }

        Ok(())
    }

    /// Returns the current state of the treasury.
    pub async fn get_latest_treasury(&self) -> Result<Option<TreasuryResult>, Error> {
        self.db
            .collection::<TreasuryResult>(TreasuryDocument::COLLECTION)
            .find_one(doc! {}, FindOneOptions::builder().sort(doc! { "_id": -1 }).build())
            .await
    }
}
