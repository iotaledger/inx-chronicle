// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{
    db::collections::{LedgerUpdateByAddressRecord, LedgerUpdateByMilestoneRecord},
    types::{
        stardust::{block::Address, milestone::MilestoneTimestamp},
        tangle::MilestoneIndex,
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

/// Response of GET /api/explorer/v2/blocks/{block_id}/children.
/// Returns all children of a specific block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockChildrenResponse {
    pub block_id: String,
    pub max_results: usize,
    pub count: usize,
    pub children: Vec<String>,
}

impl_success_response!(BlockChildrenResponse);
