// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::{
        collections::{AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, IndexedId, NftOutputsQuery},
        MongoDb,
    },
    types::stardust::block::{AliasId, FoundryId, NftId},
};
use mongodb::bson;

use super::{
    extractors::IndexedOutputsPagination,
    responses::{IndexerOutputResponse, IndexerOutputsResponse},
};
use crate::api::{error::ParseError, stardust::indexer::extractors::IndexedOutputsCursor, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().nest(
        "/outputs",
        Router::new()
            .route("/basic", get(indexed_outputs::<BasicOutputsQuery>))
            .nest(
                "/alias",
                Router::new()
                    .route("/", get(indexed_outputs::<AliasOutputsQuery>))
                    .route("/:alias_id", get(indexed_output_by_id::<AliasId>)),
            )
            .nest(
                "/foundry",
                Router::new()
                    .route("/", get(indexed_outputs::<FoundryOutputsQuery>))
                    .route("/:foundry_id", get(indexed_output_by_id::<FoundryId>)),
            )
            .nest(
                "/nft",
                Router::new()
                    .route("/", get(indexed_outputs::<NftOutputsQuery>))
                    .route("/:nft_id", get(indexed_output_by_id::<NftId>)),
            ),
    )
}

async fn indexed_output_by_id<ID>(
    database: Extension<MongoDb>,
    Path(id): Path<String>,
) -> ApiResult<IndexerOutputResponse>
where
    ID: Into<IndexedId> + FromStr,
    ParseError: From<ID::Err>,
{
    let id = ID::from_str(&id).map_err(ApiError::bad_parse)?;
    let res = database
        .get_indexed_output_by_id(id)
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
        include_spent,
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
            cursor,
            sort,
            include_spent,
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
