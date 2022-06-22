// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::{collections::SortOrder, MongoDb},
    types::stardust::block::Address,
};
use futures::{StreamExt, TryStreamExt};

use super::{
    extractors::HistoryPagination,
    responses::{TransactionHistoryResponse, Transfer},
};
use crate::api::{routes::sync, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().route("/gaps", get(sync)).nest(
        "/ledger",
        Router::new().route("/updates/by-address/:address", get(transaction_history)),
    )
}

async fn transaction_history(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    HistoryPagination {
        page_size,
        start_milestone_index,
        start_output_id,
    }: HistoryPagination,
) -> ApiResult<TransactionHistoryResponse> {
    let address_dto = Address::from_str(&address).map_err(ApiError::bad_parse)?;

    let mut records_iter = database
        .get_ledger_updates(
            &address_dto,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            start_milestone_index.map(Into::into),
            start_output_id,
            // TODO: Allow specifying sort in query
            SortOrder::Newest,
        )
        .await?;

    // Take all of the requested records first
    let records = records_iter.by_ref().take(page_size).try_collect::<Vec<_>>().await?;
    // If any record is left, use it to make the cursor
    let cursor = records_iter
        .try_next()
        .await?
        .map(|doc| format!("{}.{}.{}", doc.at.milestone_index, doc.output_id.to_hex(), page_size));

    let transactions = records
        .into_iter()
        .map(|rec| {
            Ok(Transfer {
                output_id: rec.output_id.to_hex(),
                is_spent: rec.is_spent,
                milestone_index: rec.at.milestone_index,
                milestone_timestamp: rec.at.milestone_timestamp,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionHistoryResponse {
        items: transactions,
        address,
        cursor,
    })
}
