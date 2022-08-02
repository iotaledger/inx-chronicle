// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Extension, Router};
use chronicle::db::MongoDb;

use super::{
    extractors::RichlistQuery,
    responses::{
        AddressAnalyticsResponse, BlockAnalyticsResponse, OutputAnalyticsResponse, RichlistAnalyticsResponse,
        StorageDepositAnalyticsResponse,
    },
};
use crate::api::{extractors::TimeRange, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .route("/addresses", get(address_analytics))
        .route("/storage-deposit", get(storage_deposit_analytics))
        .route("/richlist", get(richlist_analytics))
        .nest(
            "/blocks",
            Router::new()
                .route("/transaction", get(transaction_analytics))
                .route("/milestone", get(milestone_analytics))
                .route("/tagged_data", get(tagged_data_analytics)),
        )
        .nest(
            "/outputs",
            Router::new()
                .route("/basic", get(basic_analytics))
                .route("/alias", get(alias_analytics))
                .route("/nft", get(nft_analytics))
                .route("/foundry", get(foundry_analytics)),
        )
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
        total_active_addresses: res.total_active_addresses.to_string(),
        receiving_addresses: res.receiving_addresses.to_string(),
        sending_addresses: res.sending_addresses.to_string(),
    })
}

async fn transaction_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database
        .get_transaction_analytics(start_timestamp, end_timestamp)
        .await?;

    Ok(OutputAnalyticsResponse {
        count: res.count.to_string(),
        total_value: res.total_value,
    })
}

async fn milestone_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<BlockAnalyticsResponse> {
    let res = database.get_milestone_analytics(start_timestamp, end_timestamp).await?;

    Ok(BlockAnalyticsResponse {
        count: res.count.to_string(),
    })
}

async fn tagged_data_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<BlockAnalyticsResponse> {
    let res = database
        .get_tagged_data_analytics(start_timestamp, end_timestamp)
        .await?;

    Ok(BlockAnalyticsResponse {
        count: res.count.to_string(),
    })
}

async fn basic_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database.get_basic_analytics(start_timestamp, end_timestamp).await?;

    Ok(OutputAnalyticsResponse {
        count: res.count.to_string(),
        total_value: res.total_value,
    })
}

async fn alias_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database.get_alias_analytics(start_timestamp, end_timestamp).await?;

    Ok(OutputAnalyticsResponse {
        count: res.count.to_string(),
        total_value: res.total_value,
    })
}

async fn nft_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database.get_nft_analytics(start_timestamp, end_timestamp).await?;

    Ok(OutputAnalyticsResponse {
        count: res.count.to_string(),
        total_value: res.total_value,
    })
}

async fn foundry_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database.get_foundry_analytics(start_timestamp, end_timestamp).await?;

    Ok(OutputAnalyticsResponse {
        count: res.count.to_string(),
        total_value: res.total_value,
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
        .get_storage_deposit_analytics(start_timestamp, end_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(StorageDepositAnalyticsResponse {
        output_count: res.output_count.to_string(),
        storage_deposit_return_count: res.storage_deposit_return_count.to_string(),
        storage_deposit_return_total_value: res.storage_deposit_return_total_value,
        total_key_bytes: res.total_key_bytes,
        total_data_bytes: res.total_data_bytes,
        total_byte_cost: res.total_byte_cost,
        ledger_index: res.ledger_index.0,
        rent_structure: res.rent_structure.into(),
    })
}

async fn richlist_analytics(
    database: Extension<MongoDb>,
    RichlistQuery { top }: RichlistQuery,
) -> ApiResult<RichlistAnalyticsResponse> {
    let res = database.get_richlist_analytics(top).await?;

    Ok(RichlistAnalyticsResponse {
        distribution: res.distribution.into_iter().map(Into::into).collect(),
        top: res.top.into_iter().map(Into::into).collect(),
    })
}
