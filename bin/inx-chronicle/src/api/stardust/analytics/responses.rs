// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use bee_block_stardust::address::dto::AddressDto;
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

/// Response of `GET /api/analytics/addresses[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressAnalyticsResponse {
    pub total_active_addresses: u64,
    pub receiving_addresses: u64,
    pub sending_addresses: u64,
}

impl_success_response!(AddressAnalyticsResponse);

/// Response of `GET /api/analytics/transactions[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputsAnalyticsResponse {
    pub count: u64,
    pub total_value: f64,
    pub average_value: f64,
}

impl_success_response!(OutputsAnalyticsResponse);

/// Response of `GET /api/analytics/richlist[?top=<usize>]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RichlistAnalyticsResponse {
    pub distribution: Vec<DistributionStat>,
    pub top: Vec<AddressStat>,
}

impl_success_response!(RichlistAnalyticsResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressStat {
    pub address: AddressDto,
    pub balance: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionStat {
    pub range: Range<u64>,
    pub address_count: u64,
    pub total_balance: f64,
}
