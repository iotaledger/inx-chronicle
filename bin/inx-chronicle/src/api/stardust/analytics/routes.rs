// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Extension, Router};
use chronicle::db::MongoDb;

use super::responses::{AddressAnalyticsResponse, StorageDepositAnalyticsResponse, TransactionsAnalyticsResponse};
use crate::api::{extractors::TimeRange, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .route("/addresses", get(address_analytics))
        .route("/token-transfers", get(token_transfer_analytics))
        .route("/storage-deposit", get(storage_deposit_analytics))
}

async fn address_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<AddressAnalyticsResponse> {
    let res = database
        .get_address_analytics(start_timestamp, end_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(AddressAnalyticsResponse {
        total_addresses: res.total_addresses,
        receiving_addresses: res.recv_addresses,
        sending_addresses: res.send_addresses,
    })
}

async fn token_transfer_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<TransactionsAnalyticsResponse> {
    let res = database
        .get_transaction_analytics(start_timestamp, end_timestamp)
        .await?;

    Ok(TransactionsAnalyticsResponse {
        count: res.count,
        total_value: res.total_value,
        average_value: res.avg_value,
    })
}

async fn storage_deposit_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<StorageDepositAnalyticsResponse> {
    let res = database
        .get_sdruc_analytics(start_timestamp, end_timestamp)
        .await?;

    Ok(StorageDepositAnalyticsResponse {
        sdruc_amount: res.sdruc_amount,
    })
}
