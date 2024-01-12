// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{
        mongodb::collections::{LedgerUpdateCollection, OutputCollection},
        MongoDb, MongoDbCollection, MongoDbCollectionExt,
    },
    model::{
        ledger::{LedgerOutput, LedgerSpent, RentStructureBytes},
        metadata::OutputMetadata,
        utxo::{Output, OutputId},
    },
};
use futures::{prelude::stream::TryStreamExt, StreamExt};
use mongodb::bson::doc;
use serde::Deserialize;
use tokio::{task::JoinSet, try_join};

use super::Migration;

const INSERT_BATCH_SIZE: usize = 1000;

pub struct Migrate;

#[async_trait]
impl Migration for Migrate {
    const ID: usize = 2;
    const APP_VERSION: &'static str = "1.0.0-rc.3";
    const DATE: time::Date = time::macros::date!(2024 - 01 - 12);

    async fn migrate(db: &MongoDb) -> eyre::Result<()> {
        db.collection::<LedgerUpdateCollection>()
            .collection()
            .drop(None)
            .await?;

        let outputs_stream = db
            .collection::<OutputCollection>()
            .find::<OutputDocument>(doc! {}, None)
            .await?;
        let mut batched_stream = outputs_stream.try_chunks(INSERT_BATCH_SIZE);

        let mut tasks = JoinSet::new();

        while let Some(batch) = batched_stream.next().await {
            let batch = batch?;
            while tasks.len() >= 100 {
                if let Some(res) = tasks.join_next().await {
                    res??;
                }
            }
            let db = db.clone();
            tasks.spawn(async move {
                let consumed = batch.iter().filter_map(Option::<LedgerSpent>::from).collect::<Vec<_>>();
                let created = batch.into_iter().map(LedgerOutput::from).collect::<Vec<_>>();
                try_join! {
                    async {
                        db.collection::<LedgerUpdateCollection>()
                                .insert_unspent_ledger_updates(&created)
                                .await
                    },
                    async {
                        db.collection::<OutputCollection>().update_spent_outputs(&consumed).await
                    },
                    async {
                        db.collection::<LedgerUpdateCollection>().insert_spent_ledger_updates(&consumed).await
                    }
                }
                .and(Ok(()))
            });
        }

        while let Some(res) = tasks.join_next().await {
            res??;
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct OutputDocument {
    #[serde(rename = "_id")]
    output_id: OutputId,
    output: Output,
    metadata: OutputMetadata,
    details: OutputDetails,
}

#[derive(Deserialize)]
struct OutputDetails {
    rent_structure: RentStructureBytes,
}

impl From<OutputDocument> for LedgerOutput {
    fn from(value: OutputDocument) -> Self {
        Self {
            output_id: value.output_id,
            block_id: value.metadata.block_id,
            booked: value.metadata.booked,
            output: value.output,
            rent_structure: value.details.rent_structure,
        }
    }
}

impl From<&OutputDocument> for Option<LedgerSpent> {
    fn from(value: &OutputDocument) -> Self {
        value.metadata.spent_metadata.map(|spent_metadata| LedgerSpent {
            spent_metadata,
            output: LedgerOutput {
                output_id: value.output_id,
                block_id: value.metadata.block_id,
                booked: value.metadata.booked,
                output: value.output.clone(),
                rent_structure: value.details.rent_structure,
            },
        })
    }
}
