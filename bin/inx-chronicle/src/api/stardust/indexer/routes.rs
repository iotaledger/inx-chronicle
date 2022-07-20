// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::{
        collections::{AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, NftOutputsQuery},
        MongoDb,
    },
    types::stardust::block::{AliasId, FoundryId, NftId},
};
use mongodb::bson;

use super::{
    extractors::IndexedOutputsPagination,
    responses::{IndexerOutputResponse, IndexerOutputsResponse},
};
use crate::api::{stardust::indexer::extractors::IndexedOutputsCursor, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().nest(
        "/outputs",
        Router::new()
            .route("/basic", get(indexed_outputs::<BasicOutputsQuery>))
            .nest(
                "/alias",
                Router::new()
                    .route("/", get(indexed_outputs::<AliasOutputsQuery>))
                    .route("/:alias_id", get(output_by_alias_id)),
            )
            .nest(
                "/foundry",
                Router::new()
                    .route("/", get(indexed_outputs::<FoundryOutputsQuery>))
                    .route("/:foundry_id", get(output_by_foundry_id)),
            )
            .nest(
                "/nft",
                Router::new()
                    .route("/", get(indexed_outputs::<NftOutputsQuery>))
                    .route("/:nft_id", get(output_by_nft_id)),
            ),
    )
}

async fn output_by_alias_id(
    database: Extension<MongoDb>,
    Path(alias_id): Path<String>,
) -> ApiResult<IndexerOutputResponse> {
    let alias_id = AliasId::from_str(&alias_id)?;
    let res = database
        .get_alias_output_by_id(alias_id)
        .await?
        .ok_or(ApiError::NoResults)?;
    Ok(IndexerOutputResponse {
        ledger_index: res.ledger_index.0,
        output_id: res.output_id.to_hex(),
    })
}

async fn output_by_foundry_id(
    database: Extension<MongoDb>,
    Path(foundry_id): Path<String>,
) -> ApiResult<IndexerOutputResponse> {
    let foundry_id = FoundryId::from_str(&foundry_id)?;
    let res = database
        .get_foundry_output_by_id(foundry_id)
        .await?
        .ok_or(ApiError::NoResults)?;
    Ok(IndexerOutputResponse {
        ledger_index: res.ledger_index.0,
        output_id: res.output_id.to_hex(),
    })
}

async fn output_by_nft_id(
    database: Extension<MongoDb>,
    Path(nft_id): Path<String>,
) -> ApiResult<IndexerOutputResponse> {
    let nft_id = NftId::from_str(&nft_id)?;
    let res = database
        .get_nft_output_by_id(nft_id)
        .await?
        .ok_or(ApiError::NoResults)?;
    Ok(IndexerOutputResponse {
        ledger_index: res.ledger_index.0,
        output_id: res.output_id.to_hex(),
    })
}

async fn indexed_outputs<Q>(
    database: Extension<MongoDb>,
    IndexedOutputsPagination {
        query,
        page_size,
        cursor,
        sort,
    }: IndexedOutputsPagination<Q>,
) -> ApiResult<IndexerOutputsResponse>
where
    bson::Document: From<Q>,
{
    let res = database
        .get_indexed_outputs(
            query,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            cursor.map(|(ms, o)| (ms.into(), o)),
            sort,
        )
        .await?
        .ok_or(ApiError::NoResults)?;

    let mut iter = res.outputs.iter();

    // Take all of the requested records first
    let items = iter.by_ref().take(page_size).map(|o| o.output_id.to_hex()).collect();

    // If any record is left, use it to make the cursor
    let cursor = iter.next().map(|rec| {
        IndexedOutputsCursor {
            milestone_index: rec.booked_index,
            output_id: rec.output_id,
            page_size,
        }
        .to_string()
    });

    Ok(IndexerOutputsResponse {
        ledger_index: res.ledger_index.0,
        items,
        cursor,
    })
}
