// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{
        mongodb::collections::{LedgerUpdateCollection, MilestoneCollection, OutputCollection},
        MongoDb, MongoDbCollection,
    },
    tangle::LedgerUpdateStore,
};
use futures::prelude::stream::TryStreamExt;
use tokio::task::JoinSet;

use super::Migration;

const INSERT_BATCH_SIZE: usize = 1000;

pub struct Migrate;

#[async_trait]
impl Migration for Migrate {
    const ID: usize = 2;
    const APP_VERSION: &'static str = "1.0.0-rc.3";
    const DATE: time::Date = time::macros::date!(2024 - 01 - 09);

    async fn migrate(db: &MongoDb) -> eyre::Result<()> {
        let start_index = db.collection::<MilestoneCollection>().get_oldest_milestone().await?;
        let end_index = db.collection::<MilestoneCollection>().get_newest_milestone().await?;

        if let (Some(start_index), Some(end_index)) = (start_index, end_index) {
            if end_index.milestone_index > start_index.milestone_index {
                // Drop the ledger updates before we rebuild them
                db.collection::<LedgerUpdateCollection>()
                    .collection()
                    .drop(None)
                    .await?;

                // Restore the ledger updates using output data
                for index in start_index.milestone_index.0..=end_index.milestone_index.0 {
                    let consumed = db
                        .collection::<OutputCollection>()
                        .get_consumed_outputs(index.into())
                        .await?
                        .try_collect()
                        .await?;

                    let created = db
                        .collection::<OutputCollection>()
                        .get_created_outputs(index.into())
                        .await?
                        .try_collect()
                        .await?;

                    let ledger_updates = LedgerUpdateStore::init(consumed, created);

                    let mut tasks = JoinSet::new();

                    for batch in ledger_updates.created_outputs().chunks(INSERT_BATCH_SIZE) {
                        let db = db.clone();
                        let batch = batch.to_vec();
                        tasks.spawn(async move {
                            db.collection::<LedgerUpdateCollection>()
                                .insert_unspent_ledger_updates(&batch)
                                .await
                        });
                    }

                    for batch in ledger_updates.consumed_outputs().chunks(INSERT_BATCH_SIZE) {
                        let db = db.clone();
                        let batch = batch.to_vec();
                        tasks.spawn(async move {
                            db.collection::<LedgerUpdateCollection>()
                                .insert_spent_ledger_updates(&batch)
                                .await
                        });
                    }

                    while let Some(res) = tasks.join_next().await {
                        res??;
                    }
                }
            }
        }
        Ok(())
    }
}
