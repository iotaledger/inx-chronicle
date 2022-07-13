// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

/// Response of `GET /api/analytics/v2/addresses[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressAnalyticsResponse {
    pub total_addresses: u64,
    pub receiving_addresses: u64,
    pub sending_addresses: u64,
}

impl_success_response!(AddressAnalyticsResponse);

/// Response of `GET /api/analytics/v2/token-transfers[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsAnalyticsResponse {
    pub count: u64,
    pub total_value: f64,
    pub average_value: f64,
}

impl_success_response!(TransactionsAnalyticsResponse);

/// Response of `GET /api/analytics/v2/storage-deposit[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDepositAnalyticsResponse {
    // The amount of tokens locked in
    // [`UnlockCondition::StorageDepositReturn`](chronicle::types::stardust::block::output::unlock_condition::UnlockCondition::StorageDepositReturn).
    pub sdruc_amount: f64,
}

impl_success_response!(StorageDepositAnalyticsResponse);
