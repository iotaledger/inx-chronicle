// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Extension, Router};
use chronicle::db::MongoDb;

use super::responses::{AddressAnalyticsResponse, TransactionsAnalyticsResponse};
use crate::api::{extractors::TimeRange, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .route("/addresses", get(address_analytics))
        .route("/transactions", get(transaction_analytics))
}

async fn address_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<AddressAnalyticsResponse> {
    let start_milestone = database
        .find_first_milestone(start_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;
    let end_milestone = database
        .find_last_milestone(end_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;

    let res = database
        .get_address_analytics(start_milestone, end_milestone)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(AddressAnalyticsResponse {
        total_addresses: res.total_addresses,
        recv_addresses: res.recv_addresses,
        send_addresses: res.send_addresses,
    })
}

async fn transaction_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<TransactionsAnalyticsResponse> {
    let start_milestone = database
        .find_first_milestone(start_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;
    let end_milestone = database
        .find_last_milestone(end_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;

    let res = database
        .get_transaction_analytics(start_milestone, end_milestone)
        .await?;

    Ok(TransactionsAnalyticsResponse {
        count: res.count,
        total_value: res.total_value,
        avg_value: res.avg_value,
    })
}
