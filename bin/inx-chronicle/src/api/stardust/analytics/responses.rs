// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

/// Response of `GET /api/analytics/addresses[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressAnalyticsResponse {
    pub total_addresses: u64,
    pub receiving_addresses: u64,
    pub sending_addresses: u64,
}

impl_success_response!(AddressAnalyticsResponse);

/// Response of `GET /api/analytics/transactions[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsAnalyticsResponse {
    pub count: u64,
    pub total_value: f64,
    pub average_value: f64,
}

impl_success_response!(TransactionsAnalyticsResponse);
