// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollection, MongoDbCollectionExt},
    types::stardust::block::output::{AliasId, NftId, OutputId},
};
use futures::TryStreamExt;
use mongodb::{bson::doc, options::IndexOptions, IndexModel};
use serde::Deserialize;

pub const PREV_VERSION: &str = "1.0.0-beta.30";

pub async fn migrate(db: &MongoDb) -> eyre::Result<()> {
    let collection = db.collection::<OutputCollection>();

    #[derive(Deserialize)]
    struct Res {
        output_id: OutputId,
    }

    // Convert the outputs with implicit IDs
    let outputs = collection
        .aggregate::<Res>(
            [
                doc! { "$match": { "$or": [
                    { "output.alias_id": AliasId::implicit() },
                    { "output.nft_id": NftId::implicit() }
                ] } },
                doc! { "$project": {
                    "output_id": "$_id"
                } },
            ],
            None,
        )
        .await?
        .map_ok(|res| res.output_id)
        .try_collect::<Vec<_>>()
        .await?;

    for output_id in outputs {
        // Alias and nft are the same length so both can be done this way since they are just serialized as bytes
        let id = AliasId::from(output_id);
        collection
            .update_one(
                doc! { "_id": output_id },
                doc! { "$set": { "details.indexed_id": id } },
                None,
            )
            .await?;
    }

    // Get the outputs that don't have implicit IDs
    collection
        .update_many(
            doc! {
                "output.kind": "alias",
                "output.alias_id": { "$ne": AliasId::implicit() },
            },
            doc! { "$set": {
                "details.indexed_id": "$output.alias_id",
            } },
            None,
        )
        .await?;

    collection
        .update_many(
            doc! {
                "output.kind": "nft",
                "output.nft_id": { "$ne": NftId::implicit() },
            },
            doc! { "$set": {
                "details.indexed_id": "$output.nft_id",
            } },
            None,
        )
        .await?;

    collection
        .update_many(
            doc! { "output.kind": "foundry" },
            doc! { "$set": {
                "details.indexed_id": "$output.foundry_id",
            } },
            None,
        )
        .await?;

    collection
        .collection()
        .drop_index("output_alias_id_index", None)
        .await?;

    collection
        .collection()
        .drop_index("output_foundry_id_index", None)
        .await?;

    collection.collection().drop_index("output_nft_id_index", None).await?;

    collection
        .create_index(
            IndexModel::builder()
                .keys(doc! { "output.details.indexed_id": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_indexed_id_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.details.indexed_id": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

    Ok(())
}
