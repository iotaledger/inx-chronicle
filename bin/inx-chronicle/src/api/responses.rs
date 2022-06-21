// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{
    db::collections::SyncData,
    types::{ledger::LedgerInclusionState, tangle::MilestoneIndex},
};
use derive_more::From;
use serde::{Deserialize, Serialize};

macro_rules! impl_success_response {
    ($($type:ty),*) => {
        $(
            impl axum::response::IntoResponse for $type {
                fn into_response(self) -> axum::response::Response {
                    axum::Json(self).into_response()
                }
            }
        )*
    };
}

pub(crate) use impl_success_response;
use serde_json::Value;

/// Response of `GET /api/info`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    #[serde(rename = "isHealthy")]
    pub is_healthy: bool,
}

impl_success_response!(InfoResponse);

/// An aggregation type that represents the ranges of completed milestones and gaps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncDataDto(pub SyncData);

impl_success_response!(SyncDataDto);

#[derive(Clone, Debug, Serialize, Deserialize, From)]
#[serde(untagged)]
pub enum Expansion {
    Simple(String),
    Expanded(Record),
}

impl std::fmt::Display for Expansion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Expansion::Simple(s) => write!(f, "{}", s),
            Expansion::Expanded(rec) => write!(
                f,
                "id:{}{}{}",
                rec.id,
                rec.inclusion_state
                    .map_or(String::new(), |s| format!(";inclusion_state:{}", s as u8)),
                rec.milestone_index
                    .map_or(String::new(), |i| format!(";milestone_index:{}", i)),
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    #[serde(rename = "inclusionState")]
    pub inclusion_state: Option<LedgerInclusionState>,
    #[serde(rename = "milestoneIndex")]
    pub milestone_index: Option<MilestoneIndex>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transfer {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "outputIndex")]
    pub output_index: u16,
    #[serde(rename = "isSpending")]
    pub is_spending: bool,
    #[serde(rename = "inclusionState")]
    pub inclusion_state: Option<LedgerInclusionState>,
    #[serde(rename = "blockId")]
    pub block_id: String,
    pub amount: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaybeSpentOutput {
    pub output: Value,
    #[serde(rename = "spendingBlockId")]
    pub spending_block_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unlock {
    #[serde(rename = "blockId")]
    pub block_id: String,
    pub block: Value,
}
