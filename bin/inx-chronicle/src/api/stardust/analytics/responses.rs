// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::api::impl_success_response;

/// Response of `GET /api/analytics/addresses[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct AddressAnalyticsResponse {
    #[serde(rename = "totalAddresses")]
    pub total_addresses: u64,
    #[serde(rename = "receivingAddresses")]
    pub recv_addresses: u64,
    #[serde(rename = "sendingAddresses")]
    pub send_addresses: u64,
}

impl_success_response!(AddressAnalyticsResponse);

/// Response of `GET /api/analytics/transactions[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TransactionsAnalyticsResponse {
    pub count: u64,
    #[serde(rename = "totalValue")]
    pub total_value: f64,
    #[serde(rename = "averageValue")]
    pub avg_value: f64,
}

impl_success_response!(TransactionsAnalyticsResponse);
