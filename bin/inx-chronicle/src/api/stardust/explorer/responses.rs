// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::response::IntoResponse;
use chronicle::types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex};
use serde::{Deserialize, Serialize};

use crate::api::impl_success_response;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionHistoryResponse {
    pub address: String,
    pub transactions: Vec<Transfer>,
    pub paging_state: Option<String>,
}

impl_success_response!(TransactionHistoryResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transfer {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "outputIndex")]
    pub output_index: u16,
    #[serde(rename = "isSpent")]
    pub is_spent: bool,
    #[serde(rename = "milestoneIndex")]
    pub milestone_index: MilestoneIndex,
    #[serde(rename = "milestoneTimestamp")]
    pub milestone_timestamp: Option<MilestoneTimestamp>,
}
