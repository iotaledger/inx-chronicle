// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{db::MongoDb, types::stardust::block::AliasId};

use super::{
    extractors::{AliasOutputsPagination, BasicOutputsPagination},
    responses::{IndexerOutputResponse, IndexerOutputsResponse},
};
use crate::api::{ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().nest(
        "/outputs",
        Router::new().route("/basic", get(basic_outputs)).nest(
            "/alias",
            Router::new()
                .route("/", get(alias_outputs))
                .route("/:alias_id", get(output_by_alias_id)),
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
    let mut iter = res.outputs.iter();

    // Take all of the requested records first
    let items = iter.by_ref().take(page_size).map(|o| o.output_id.to_hex()).collect();

    // If any record is left, use it to make the cursor
    let cursor = iter
        .next()
        .map(|rec| format!("{}.{}.{}", rec.booked_index, rec.output_id.to_hex(), page_size));

    Ok(IndexerOutputsResponse {
        ledger_index: res.ledger_index.0,
        items,
        cursor,
    })
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
    let mut iter = res.outputs.iter();

    // Take all of the requested records first
    let items = iter.by_ref().take(page_size).map(|o| o.output_id.to_hex()).collect();

    // If any record is left, use it to make the cursor
    let cursor = iter
        .next()
        .map(|rec| format!("{}.{}.{}", rec.booked_index, rec.output_id.to_hex(), page_size));

    Ok(IndexerOutputsResponse {
        ledger_index: res.ledger_index.0,
        items,
        cursor,
    })
}
