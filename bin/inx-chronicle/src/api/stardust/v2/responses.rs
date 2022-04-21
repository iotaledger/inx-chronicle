// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::response::IntoResponse;
use chronicle::db::model::inclusion_state::LedgerInclusionState;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{
    impl_success_response,
    responses::{Expansion, SuccessBody},
};

/// Response of `GET /api/v2/messages/<message_id>`
/// and `GET /api/v2/transactions/<transaction_id>/included-message`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u8,
    #[serde(rename = "parentMessageIds")]
    pub parents: Vec<String>,
    pub payload: Option<Value>,
    pub nonce: u64,
}

impl_success_response!(MessageResponse);

/// Response of `GET /api/v2/messages/<message_id>/metadata`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageMetadataResponse {
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "parentMessageIds")]
    pub parent_message_ids: Vec<String>,
    #[serde(rename = "isSolid")]
    pub is_solid: Option<bool>,
    #[serde(rename = "referencedByMilestoneIndex", skip_serializing_if = "Option::is_none")]
    pub referenced_by_milestone_index: Option<u32>,
    #[serde(rename = "milestoneIndex", skip_serializing_if = "Option::is_none")]
    pub milestone_index: Option<u32>,
    #[serde(rename = "ledgerInclusionState", skip_serializing_if = "Option::is_none")]
    pub ledger_inclusion_state: Option<LedgerInclusionState>,
    #[serde(rename = "conflictReason", skip_serializing_if = "Option::is_none")]
    pub conflict_reason: Option<u8>,
    #[serde(rename = "shouldPromote", skip_serializing_if = "Option::is_none")]
    pub should_promote: Option<bool>,
    #[serde(rename = "shouldReattach", skip_serializing_if = "Option::is_none")]
    pub should_reattach: Option<bool>,
}

impl_success_response!(MessageMetadataResponse);

/// Response of `GET /api/v2/messages/<message_id>/children`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageChildrenResponse {
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "maxResults")]
    pub max_results: usize,
    pub count: usize,
    #[serde(rename = "childrenMessageIds")]
    pub children_message_ids: Vec<Expansion>,
}

impl_success_response!(MessageChildrenResponse);

/// Response of `GET /api/v2/outputs/<output_id>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputResponse {
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "outputIndex")]
    pub output_index: u16,
    #[serde(rename = "spendingTransaction")]
    pub spending_transaction: Option<Value>,
    pub output: Value,
}

impl_success_response!(OutputResponse);

/// Response of `GET /api/v2/transactions/<message_id>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// The created output's message id
    #[serde(rename = "messageId")]
    pub message_id: String,
    /// The confirmation timestamp
    #[serde(rename = "milestoneIndex")]
    pub milestone_index: Option<u32>,
    /// The output
    pub outputs: Vec<Value>,
    /// The inputs, if they exist
    pub inputs: Vec<Value>,
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
pub struct MilestoneResponse {
    #[serde(rename = "index")]
    pub milestone_index: u32,
    #[serde(rename = "messageId")]
    pub message_id: String,
    pub timestamp: u32,
}

impl_success_response!(MilestoneResponse);
