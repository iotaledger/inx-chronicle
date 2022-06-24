// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::{
    stardust::{block::Address, milestone::MilestoneTimestamp},
    tangle::MilestoneIndex,
};
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionsPerAddressResponse {
    pub address: String,
    pub items: Vec<TransferByAddress>,
    pub cursor: Option<String>,
}

impl_success_response!(TransactionsPerAddressResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferByAddress {
    #[serde(rename = "outputId")]
    pub output_id: String,
    #[serde(rename = "isSpent")]
    pub is_spent: bool,
    #[serde(rename = "milestoneIndex")]
    pub milestone_index: MilestoneIndex,
    #[serde(rename = "milestoneTimestamp")]
    pub milestone_timestamp: MilestoneTimestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionsPerMilestoneResponse {
    pub index: MilestoneIndex,
    pub items: Vec<TransferByMilestone>,
    pub cursor: Option<String>,
}

impl_success_response!(TransactionsPerMilestoneResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferByMilestone {
    pub address: Address,
    #[serde(rename = "outputId")]
    pub output_id: String,
    #[serde(rename = "isSpent")]
    pub is_spent: bool,
}
