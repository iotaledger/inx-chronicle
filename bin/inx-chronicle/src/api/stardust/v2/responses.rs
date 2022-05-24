// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::response::IntoResponse;
use chronicle::db::model::{
    ledger::LedgerInclusionState,
    stardust::{
        block::{Input, Output, Payload},
        milestone::MilestoneTimestamp,
    },
    tangle::MilestoneIndex,
};
use serde::{Deserialize, Serialize};

use crate::api::{impl_success_response, responses::Expansion};

/// Response of `GET /api/v2/blocks/<block_id>`
/// and `GET /api/v2/transactions/<transaction_id>/included-block`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockResponse {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u8,
    pub parents: Vec<String>,
    pub payload: Option<Payload>,
    pub nonce: u64,
}

impl_success_response!(BlockResponse);

/// Response of `GET /api/v2/blocks/<block_id>/metadata`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockMetadataResponse {
    #[serde(rename = "blockId")]
    pub block_id: String,
    #[serde(rename = "parentBlockIds")]
    pub parents: Vec<String>,
    #[serde(rename = "isSolid")]
    pub is_solid: Option<bool>,
    #[serde(rename = "referencedByMilestoneIndex", skip_serializing_if = "Option::is_none")]
    pub referenced_by_milestone_index: Option<MilestoneIndex>,
    #[serde(rename = "milestoneIndex", skip_serializing_if = "Option::is_none")]
    pub milestone_index: Option<MilestoneIndex>,
    #[serde(rename = "ledgerInclusionState", skip_serializing_if = "Option::is_none")]
    pub ledger_inclusion_state: Option<LedgerInclusionState>,
    #[serde(rename = "conflictReason", skip_serializing_if = "Option::is_none")]
    pub conflict_reason: Option<u8>,
    #[serde(rename = "shouldPromote", skip_serializing_if = "Option::is_none")]
    pub should_promote: Option<bool>,
    #[serde(rename = "shouldReattach", skip_serializing_if = "Option::is_none")]
    pub should_reattach: Option<bool>,
}

impl_success_response!(BlockMetadataResponse);

/// Response of `GET /api/v2/blocks/<block_id>/children`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockChildrenResponse {
    #[serde(rename = "blockId")]
    pub block_id: String,
    #[serde(rename = "maxResults")]
    pub max_results: usize,
    pub count: usize,
    pub children: Vec<Expansion>,
}

impl_success_response!(BlockChildrenResponse);

/// Response of `GET /api/v2/outputs/<output_id>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputResponse {
    #[serde(rename = "blockId")]
    pub block_id: String,
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "outputIndex")]
    pub output_index: u16,
    #[serde(rename = "spendingTransaction")]
    pub is_spent: bool,
    #[serde(rename = "milestoneIndexSpent")]
    pub milestone_index_spent: Option<MilestoneIndex>,
    #[serde(rename = "milestoneTimestampSpent")]
    pub milestone_ts_spent: Option<MilestoneTimestamp>,
    #[serde(rename = "milestoneIndexBooked")]
    pub milestone_index_booked: MilestoneIndex,
    #[serde(rename = "milestoneTimestampBooked")]
    pub milestone_ts_booked: MilestoneTimestamp,
    pub output: Output,
}

impl_success_response!(OutputResponse);

/// Response of `GET /api/v2/outputs/<output_id>/metadata`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputMetadataResponse {
    #[serde(rename = "blockId")]
    pub block_id: String,
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "outputIndex")]
    pub output_index: u16,
    #[serde(rename = "spendingTransaction")]
    pub is_spent: bool,
    #[serde(rename = "milestoneIndexSpent")]
    pub milestone_index_spent: Option<MilestoneIndex>,
    #[serde(rename = "milestoneTimestampSpent")]
    pub milestone_ts_spent: Option<MilestoneTimestamp>,
    #[serde(rename = "transactionIdSpent")]
    pub transaction_id_spent: Option<String>,
    #[serde(rename = "milestoneIndexBooked")]
    pub milestone_index_booked: MilestoneIndex,
    #[serde(rename = "milestoneTimestampBooked")]
    pub milestone_ts_booked: MilestoneTimestamp,
}

impl_success_response!(OutputMetadataResponse);

/// Response of `GET /api/v2/transactions/<block_id>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// The created output's block id
    #[serde(rename = "blockId")]
    pub block_id: String,
    /// The confirmation timestamp
    #[serde(rename = "milestoneIndex")]
    pub milestone_index: Option<MilestoneIndex>,
    /// The output
    pub outputs: Vec<Output>,
    /// The inputs, if they exist
    pub inputs: Vec<Input>,
}

impl_success_response!(TransactionResponse);

/// Response of `GET /api/v2/transactions/ed25519/<address>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionsResponse {
    pub transactions: Vec<TransactionResponse>,
}

impl_success_response!(TransactionsResponse);

/// Response of `GET /api/v2/milestone/<index>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MilestoneResponse {
    pub payload: Payload,
}

impl_success_response!(MilestoneResponse);
