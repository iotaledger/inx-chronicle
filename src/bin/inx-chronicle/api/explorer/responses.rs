// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use chronicle::{
    db::mongodb::collections::{
        DistributionStat, LedgerUpdateByAddressRecord, LedgerUpdateByMilestoneRecord, MilestoneResult,
    },
    model::{
        tangle::{MilestoneIndex, MilestoneTimestamp},
        utxo::Address,
    },
};
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdatesByAddressResponse {
    pub address: String,
    pub items: Vec<LedgerUpdateByAddressDto>,
    pub cursor: Option<String>,
}

impl_success_response!(LedgerUpdatesByAddressResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdateByAddressDto {
    pub output_id: String,
    pub is_spent: bool,
    pub milestone_index: MilestoneIndex,
    pub milestone_timestamp: MilestoneTimestamp,
}

impl From<LedgerUpdateByAddressRecord> for LedgerUpdateByAddressDto {
    fn from(value: LedgerUpdateByAddressRecord) -> Self {
        Self {
            output_id: value.output_id.to_hex(),
            is_spent: value.is_spent,
            milestone_index: value.at.milestone_index,
            milestone_timestamp: value.at.milestone_timestamp,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdatesByMilestoneResponse {
    pub milestone_index: MilestoneIndex,
    pub items: Vec<LedgerUpdateByMilestoneDto>,
    pub cursor: Option<String>,
}

impl_success_response!(LedgerUpdatesByMilestoneResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdateByMilestoneDto {
    pub address: Address,
    pub output_id: String,
    pub is_spent: bool,
}

impl From<LedgerUpdateByMilestoneRecord> for LedgerUpdateByMilestoneDto {
    fn from(value: LedgerUpdateByMilestoneRecord) -> Self {
        Self {
            address: value.address,
            output_id: value.output_id.to_hex(),
            is_spent: value.is_spent,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    pub total_balance: String,
    pub sig_locked_balance: String,
    pub ledger_index: MilestoneIndex,
}

impl_success_response!(BalanceResponse);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockChildrenResponse {
    pub block_id: String,
    pub max_results: usize,
    pub count: usize,
    pub children: Vec<String>,
}

impl_success_response!(BlockChildrenResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestonesResponse {
    pub items: Vec<MilestoneDto>,
    pub cursor: Option<String>,
}

impl_success_response!(MilestonesResponse);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockPayloadTypeDto {
    pub block_id: String,
    #[serde(rename = "payloadType")]
    pub payload_kind: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlocksByMilestoneResponse {
    pub blocks: Vec<BlockPayloadTypeDto>,
    pub cursor: Option<String>,
}

impl_success_response!(BlocksByMilestoneResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneDto {
    milestone_id: String,
    index: MilestoneIndex,
}

impl From<MilestoneResult> for MilestoneDto {
    fn from(res: MilestoneResult) -> Self {
        Self {
            milestone_id: res.milestone_id.to_hex(),
            index: res.index,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RichestAddressesResponse {
    pub top: Vec<AddressStatDto>,
    pub ledger_index: MilestoneIndex,
}

impl_success_response!(RichestAddressesResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressStatDto {
    pub address: String,
    pub balance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDistributionResponse {
    pub distribution: Vec<DistributionStatDto>,
    pub ledger_index: MilestoneIndex,
}

impl_success_response!(TokenDistributionResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionStatDto {
    pub range: Range<u64>,
    pub address_count: String,
    pub total_balance: String,
}

impl From<DistributionStat> for DistributionStatDto {
    fn from(s: DistributionStat) -> Self {
        Self {
            range: 10_u64.pow(s.index)..10_u64.pow(s.index + 1),
            address_count: s.address_count.to_string(),
            total_balance: s.total_balance,
        }
    }
}
