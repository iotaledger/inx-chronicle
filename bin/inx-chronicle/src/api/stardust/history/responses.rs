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
pub struct LederUpdatesByAddressResponse {
    pub address: String,
    pub items: Vec<LedgerUpdateByAddressResponse>,
    pub cursor: Option<String>,
}

impl_success_response!(LederUpdatesByAddressResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdateByAddressResponse {
    pub output_id: String,
    pub is_spent: bool,
    pub milestone_index: MilestoneIndex,
    pub milestone_timestamp: MilestoneTimestamp,
}

impl From<LedgerUpdateByAddressRecord> for LedgerUpdateByAddressResponse {
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
    pub items: Vec<LedgerUpdateByMilestoneResponse>,
    pub cursor: Option<String>,
}

impl_success_response!(LedgerUpdatesByMilestoneResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdateByMilestoneResponse {
    pub address: Address,
    pub output_id: String,
    pub is_spent: bool,
}

impl From<LedgerUpdateByMilestoneRecord> for LedgerUpdateByMilestoneResponse {
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
