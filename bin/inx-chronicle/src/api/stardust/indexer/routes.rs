// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Extension, Router};
use chronicle::db::MongoDb;

use super::{extractors::BasicOutputsPagination, responses::BasicOutputsResponse};
use crate::api::{ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().route("/outputs/basic", get(basic_outputs))
}

async fn basic_outputs(
    database: Extension<MongoDb>,
    BasicOutputsPagination {
        query,
        page_size,
        cursor,
    }: BasicOutputsPagination,
) -> ApiResult<BasicOutputsResponse> {
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

    Ok(BasicOutputsResponse {
        ledger_index: res.ledger_index.0,
        items,
        cursor,
    })
}
