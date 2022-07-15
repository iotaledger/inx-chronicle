// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::{collections::OutputsResult, MongoDb},
    types::stardust::block::{AliasId, FoundryId, NftId},
};

use super::{
    extractors::{AliasOutputsPagination, BasicOutputsPagination, FoundryOutputsPagination, NftOutputsPagination},
    responses::{IndexerOutputResponse, IndexerOutputsResponse},
};
use crate::api::{ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().nest(
        "/outputs",
        Router::new()
            .route("/basic", get(basic_outputs))
            .nest(
                "/alias",
                Router::new()
                    .route("/", get(alias_outputs))
                    .route("/:alias_id", get(output_by_alias_id)),
            )
            .nest(
                "/foundry",
                Router::new()
                    .route("/", get(foundry_outputs))
                    .route("/:foundry_id", get(output_by_foundry_id)),
            )
            .nest(
                "/nft",
                Router::new()
                    .route("/", get(nft_outputs))
                    .route("/:nft_id", get(output_by_nft_id)),
            ),
    )
}

async fn basic_outputs(
    database: Extension<MongoDb>,
    BasicOutputsPagination {
        query,
        page_size,
        cursor,
    }: BasicOutputsPagination,
) -> ApiResult<IndexerOutputsResponse> {
    let res = database
        .get_basic_outputs(
            query,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            cursor.map(|(ms, o)| (ms.into(), o)),
        )
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(create_outputs_response(res, page_size))
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

async fn alias_outputs(
    database: Extension<MongoDb>,
    AliasOutputsPagination {
        query,
        page_size,
        cursor,
    }: AliasOutputsPagination,
) -> ApiResult<IndexerOutputsResponse> {
    let res = database
        .get_alias_outputs(
            query,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            cursor.map(|(ms, o)| (ms.into(), o)),
        )
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(create_outputs_response(res, page_size))
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

async fn foundry_outputs(
    database: Extension<MongoDb>,
    FoundryOutputsPagination {
        query,
        page_size,
        cursor,
    }: FoundryOutputsPagination,
) -> ApiResult<IndexerOutputsResponse> {
    let res = database
        .get_foundry_outputs(
            query,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            cursor.map(|(ms, o)| (ms.into(), o)),
        )
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(create_outputs_response(res, page_size))
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

async fn nft_outputs(
    database: Extension<MongoDb>,
    NftOutputsPagination {
        query,
        page_size,
        cursor,
    }: NftOutputsPagination,
) -> ApiResult<IndexerOutputsResponse> {
    let res = database
        .get_nft_outputs(
            query,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            cursor.map(|(ms, o)| (ms.into(), o)),
        )
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(create_outputs_response(res, page_size))
}

fn create_outputs_response(res: OutputsResult, page_size: usize) -> IndexerOutputsResponse {
    let mut iter = res.outputs.iter();

    // Take all of the requested records first
    let items = iter.by_ref().take(page_size).map(|o| o.output_id.to_hex()).collect();

    // If any record is left, use it to make the cursor
    let cursor = iter
        .next()
        .map(|rec| format!("{}.{}.{}", rec.booked_index, rec.output_id.to_hex(), page_size));

    IndexerOutputsResponse {
        ledger_index: res.ledger_index.0,
        items,
        cursor,
    }
}
