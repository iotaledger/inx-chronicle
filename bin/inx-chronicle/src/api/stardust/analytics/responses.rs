// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use bee_block_stardust::address::dto::AddressDto;
use chronicle::{
    db::collections::{AddressStat, DistributionStat},
    types::tangle::RentStructure,
};
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

/// Response of `GET /api/analytics/addresses[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressAnalyticsResponse {
    pub total_active_addresses: String,
    pub receiving_addresses: String,
    pub sending_addresses: String,
}

impl_success_response!(AddressAnalyticsResponse);

/// Response of `GET /api/analytics/transactions[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputAnalyticsResponse {
    pub count: String,
    pub total_value: String,
}

impl_success_response!(OutputAnalyticsResponse);

/// Response of `GET /api/analytics/transactions[?start_timestamp=<i64>&end_timestamp=<i64>]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAnalyticsResponse {
    pub count: String,
}

impl_success_response!(BlockAnalyticsResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDepositAnalyticsResponse {
    pub output_count: String,
    pub storage_deposit_return_count: String,
    pub storage_deposit_return_total_value: String,
    pub total_key_bytes: String,
    pub total_data_bytes: String,
    pub total_byte_cost: String,
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

/// Response of `GET /api/analytics/richlist[?top=<usize>]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RichlistAnalyticsResponse {
    pub distribution: Vec<DistributionStatDto>,
    pub top: Vec<AddressStatDto>,
}

impl_success_response!(RichlistAnalyticsResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressStatDto {
    pub address: AddressDto,
    pub balance: f64,
}

impl From<AddressStat> for AddressStatDto {
    fn from(s: AddressStat) -> Self {
        Self {
            address: AddressDto::from(&bee_block_stardust::address::Address::from(s.address)),
            balance: s.balance,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionStatDto {
    pub range: Range<u64>,
    pub address_count: u64,
    pub total_balance: f64,
}

impl From<DistributionStat> for DistributionStatDto {
    fn from(s: DistributionStat) -> Self {
        Self {
            range: 10_u64.pow(s.index)..10_u64.pow(s.index + 1),
            address_count: s.address_count,
            total_balance: s.total_balance,
        }
    }
}
