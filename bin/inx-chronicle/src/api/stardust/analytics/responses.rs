// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::tangle::RentStructure;
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
}

impl_success_response!(OutputsAnalyticsResponse);

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDepositAnalyticsResponse {
    pub output_count: u64,
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_total_value: f64,
    pub total_key_bytes: f64,
    pub total_data_bytes: f64,
    pub total_byte_cost: f64,
    pub ledger_index: u32,
    pub rent_structure: RentStructureDto,
}

impl_success_response!(StorageDepositAnalyticsResponse);

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RentStructureDto {
    pub v_byte_cost: u32,
    pub v_byte_factor_data: u8,
    pub v_byte_factor_key: u8,
}

impl From<RentStructure> for RentStructureDto {
    fn from(s: RentStructure) -> Self {
        Self {
            v_byte_cost: s.v_byte_cost,
            v_byte_factor_data: s.v_byte_factor_data,
            v_byte_factor_key: s.v_byte_factor_key,
        }
    }
}
