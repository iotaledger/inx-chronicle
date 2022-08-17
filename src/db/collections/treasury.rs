// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::doc,
    error::Error,
    options::{FindOneOptions, IndexOptions},
    ClientSession, IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

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
pub struct TreasuryResult {
    pub milestone_id: MilestoneId,
    pub amount: u64,
}

/// Queries that are related to the treasury.
impl MongoDb {
    /// Creates ledger update indexes.
    pub async fn create_treasury_indexes(&self) -> Result<(), Error> {
        let collection = self.db.collection::<TreasuryDocument>(TreasuryDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "milestone_index": -1, "milestone_id": 1 })
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
        session: &mut ClientSession,
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
        if !payloads.is_empty() {
            self.db
                .collection::<TreasuryDocument>(TreasuryDocument::COLLECTION)
                .insert_many_with_session(payloads, None, session)
                .await?;
        }

        Ok(())
    }

    /// Returns the current state of the treasury.
    pub async fn get_latest_treasury(&self) -> Result<Option<TreasuryResult>, Error> {
        self.db
            .collection::<TreasuryResult>(TreasuryDocument::COLLECTION)
            .find_one(
                doc! {},
                FindOneOptions::builder().sort(doc! { "milestone_index": -1 }).build(),
            )
            .await
    }
}
