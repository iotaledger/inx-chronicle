// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Extension, Router};
use bee_block_stardust::address::{dto::AddressDto, Address};
use chronicle::db::MongoDb;

use super::{
    extractors::RichlistQuery,
    responses::{
        AddressAnalyticsResponse, AddressStat, DistributionStat, OutputsAnalyticsResponse, RichlistAnalyticsResponse,
    },
};
use crate::api::{extractors::TimeRange, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .route("/addresses", get(address_analytics))
        .route("/transactions", get(transaction_analytics))
        .route("/native-tokens", get(native_token_analytics))
        .route("/nfts", get(nft_analytics))
        .route("/foundrys", get(foundry_analytics))
        .route("/storage-deposit", get(locked_storage_deposit_analytics))
        .route("/richlist", get(richlist_analytics))
}

async fn address_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<AddressAnalyticsResponse> {
    let res = database.get_address_analytics(start_timestamp, end_timestamp).await?;

    Ok(AddressAnalyticsResponse {
        total_active_addresses: res.total_active_addresses,
        receiving_addresses: res.receiving_addresses,
        sending_addresses: res.sending_addresses,
    })
}

async fn transaction_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputsAnalyticsResponse> {
    let res = database
        .get_transaction_analytics(start_timestamp, end_timestamp)
        .await?;

    Ok(OutputsAnalyticsResponse {
        count: res.count,
        total_value: res.total_value,
        average_value: res.avg_value,
    })
}

async fn native_token_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputsAnalyticsResponse> {
    let res = database
        .get_native_token_analytics(start_timestamp, end_timestamp)
        .await?;

    Ok(OutputsAnalyticsResponse {
        count: res.count,
        total_value: res.total_value,
        average_value: res.avg_value,
    })
}

async fn nft_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputsAnalyticsResponse> {
    let res = database.get_nft_analytics(start_timestamp, end_timestamp).await?;

    Ok(OutputsAnalyticsResponse {
        count: res.count,
        total_value: res.total_value,
        average_value: res.avg_value,
    })
}

async fn foundry_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputsAnalyticsResponse> {
    let res = database.get_foundry_analytics(start_timestamp, end_timestamp).await?;

    Ok(OutputsAnalyticsResponse {
        count: res.count,
        total_value: res.total_value,
        average_value: res.avg_value,
    })
}

async fn locked_storage_deposit_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputsAnalyticsResponse> {
    let res = database
        .get_locked_storage_deposit_analytics(start_timestamp, end_timestamp)
        .await?;

    Ok(OutputsAnalyticsResponse {
        count: res.count,
        total_value: res.total_value,
        average_value: res.avg_value,
    })
}

async fn richlist_analytics(
    database: Extension<MongoDb>,
    RichlistQuery { top }: RichlistQuery,
) -> ApiResult<RichlistAnalyticsResponse> {
    let res = database.get_richlist_analytics(top).await?;

    Ok(RichlistAnalyticsResponse {
        distribution: res
            .distribution
            .into_iter()
            .map(|s| DistributionStat {
                range: 10_u64.pow(s.index)..10_u64.pow(s.index + 1),
                address_count: s.address_count,
                total_balance: s.total_balance,
            })
            .collect(),
        top: res
            .top
            .into_iter()
            .map(|s| AddressStat {
                address: AddressDto::from(&Address::from(s.address)),
                balance: s.balance,
            })
            .collect(),
    })
}
