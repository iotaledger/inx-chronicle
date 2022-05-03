// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Extension, Router};
use chronicle::db::{bson::DocExt, MongoDb};

use super::responses::AddressAnalyticsResponse;
use crate::api::{
    extractors::TimeRange,
    stardust::{end_milestone, start_milestone},
    ApiError, ApiResult,
};

pub fn routes() -> Router {
    Router::new().route("/addresses", get(address_analytics))
}

async fn address_analytics(
    database: Extension<MongoDb>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<AddressAnalyticsResponse> {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let res = database
        .aggregate_addresses(start_milestone, end_milestone)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(AddressAnalyticsResponse {
        total_addresses: res.get_as_u64("total_addresses")?,
        recv_addresses: res.get_as_u64("recv_addresses")?,
        send_addresses: res.get_as_u64("send_addresses")?,
    })
}
