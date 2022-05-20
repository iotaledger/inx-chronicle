// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{db::MongoDb, types::stardust::block::Address};
use futures::TryStreamExt;

use super::responses::{TransactionHistoryResponse, Transfer};
use crate::api::{
    error::ParseError,
    extractors::{Pagination, TimeRange},
    ApiError, ApiResult,
};

pub fn routes() -> Router {
    Router::new().nest(
        "/transactions",
        Router::new().route("/history/:address", get(transaction_history)),
    )
}

async fn transaction_history(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    Pagination { page_size, page }: Pagination,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<TransactionHistoryResponse> {
    let address_dto = Address::from_str(&address).map_err(ParseError::StorageType)?;
    let start_milestone = database
        .find_first_milestone(start_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;
    let end_milestone = database
        .find_last_milestone(end_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;

    let records = database
        .get_transaction_history(&address_dto, page_size, page, start_milestone, end_milestone)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let transactions = records
        .into_iter()
        .map(|rec| {
            Ok(Transfer {
                transaction_id: rec.transaction_id.to_hex(),
                output_index: rec.output_index,
                is_spent: rec.is_spent,
                inclusion_state: rec.inclusion_state,
                block_id: rec.block_id.to_hex(),
                amount: rec.amount,
                milestone_index: rec.milestone_index,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionHistoryResponse { transactions, address })
}
