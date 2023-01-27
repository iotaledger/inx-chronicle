// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{
    db::{
        collections::OutputCollection, mongodb::config as mongocfg, MongoDb, MongoDbCollection, MongoDbCollectionExt,
        MongoDbConfig,
    },
    types::stardust::block::output::{AliasId, NftId, OutputId},
};
use clap::Parser;
use futures::TryStreamExt;
use mongodb::{bson::doc, options::IndexOptions, IndexModel};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct MongoDbArgs {
    /// The MongoDb connection string.
    #[arg(
        long,
        value_name = "CONN_STR",
        env = "MONGODB_CONN_STR",
        default_value = mongocfg::DEFAULT_CONN_STR,
    )]
    pub mongodb_conn_str: String,
    /// The MongoDb database name.
    #[arg(long, value_name = "NAME", default_value = mongocfg::DEFAULT_DATABASE_NAME)]
    pub mongodb_database_name: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = MongoDbArgs::parse();
    let config = MongoDbConfig {
        conn_str: args.mongodb_conn_str,
        database_name: args.mongodb_database_name,
    };

    let db = MongoDb::connect(&config).await?;

    let collection = db.collection::<OutputCollection>();

    #[derive(Deserialize)]
    struct Res {
        output_id: OutputId,
    }

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
                doc! { "$_id": output_id },
                doc! { "$set": { "details.indexed_id": id } },
                None,
            )
            .await?;
    }

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
