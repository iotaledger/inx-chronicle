// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use axum::response::IntoResponse;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value;

use crate::types::LedgerInclusionState;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum ListenerResponse {
    /// Response of GET /api/<api_version>/info
    Info {
        name: String,
        version: String,
        #[serde(rename = "isHealthy")]
        is_healthy: bool,
    },
    /// Response of GET /api/<api_version>/messages/<message_id>
    /// and GET /api/<api_version>/transactions/<transaction_id>/included-message
    Message {
        #[serde(rename = "networkId")]
        network_id: Option<u64>,
        #[serde(rename = "protocolVersion")]
        protocol_version: u8,
        #[serde(rename = "parentMessageIds")]
        parents: Vec<String>,
        payload: Option<Value>,
        nonce: u64,
    },
    /// Response of GET /api/<api_version>/messages/<message_id>/metadata
    MessageMetadata {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "parentMessageIds")]
        parent_message_ids: Vec<String>,
        #[serde(rename = "isSolid")]
        is_solid: bool,
        #[serde(rename = "referencedByMilestoneIndex", skip_serializing_if = "Option::is_none")]
        referenced_by_milestone_index: Option<u32>,
        #[serde(rename = "milestoneIndex", skip_serializing_if = "Option::is_none")]
        milestone_index: Option<u32>,
        #[serde(rename = "ledgerInclusionState", skip_serializing_if = "Option::is_none")]
        ledger_inclusion_state: Option<LedgerInclusionState>,
        #[serde(rename = "conflictReason", skip_serializing_if = "Option::is_none")]
        conflict_reason: Option<u8>,
        #[serde(rename = "shouldPromote", skip_serializing_if = "Option::is_none")]
        should_promote: Option<bool>,
        #[serde(rename = "shouldReattach", skip_serializing_if = "Option::is_none")]
        should_reattach: Option<bool>,
    },
    /// Response of GET /api/<api_version>/messages/<message_id>/children
    MessageChildren {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "maxResults")]
        max_results: usize,
        count: usize,
        #[serde(rename = "childrenMessageIds")]
        children_message_ids: Vec<String>,
    },
    /// Response of GET /api/<api_version>/messages/<message_id>/children[?expanded=true]
    MessageChildrenExpanded {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "maxResults")]
        max_results: usize,
        count: usize,
        #[serde(rename = "childrenMessageIds")]
        children_message_ids: Vec<Record>,
    },
    /// Response of GET /api/<api_version>/messages?<index>
    MessagesForIndex {
        index: String,
        #[serde(rename = "maxResults")]
        max_results: usize,
        count: usize,
        #[serde(rename = "messageIds")]
        message_ids: Vec<String>,
    },
    /// Response of GET /api/<api_version>/messages?<index>[&expanded=true]
    MessagesForIndexExpanded {
        index: String,
        #[serde(rename = "maxResults")]
        max_results: usize,
        count: usize,
        #[serde(rename = "messageIds")]
        message_ids: Vec<Record>,
    },
    /// Response of GET /api/<api_version>/messages?<index>
    MessagesForTag {
        tag: String,
        #[serde(rename = "maxResults")]
        max_results: usize,
        count: usize,
        #[serde(rename = "messageIds")]
        message_ids: Vec<String>,
    },
    /// Response of GET /api/<api_version>/messages?<index>[&expanded=true]
    MessagesForTagExpanded {
        tag: String,
        #[serde(rename = "maxResults")]
        max_results: usize,
        count: usize,
        #[serde(rename = "messageIds")]
        message_ids: Vec<Record>,
    },
    /// Response of GET /api/<api_version>/addresses/<address>/outputs
    OutputsForAddress {
        address: String,
        #[serde(rename = "maxResults")]
        max_results: usize,
        count: usize,
        #[serde(rename = "outputIds")]
        output_ids: Vec<String>,
    },
    /// Response of GET /api/<api_version>/addresses/<address>/outputs[?expanded=true]
    OutputsForAddressExpanded {
        address: String,
        #[serde(rename = "maxResults")]
        max_results: usize,
        count: usize,
        #[serde(rename = "outputIds")]
        output_ids: Vec<Record>,
    },
    /// Response of GET /api/<api_version>/outputs/<output_id>
    Output {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "transactionId")]
        transaction_id: String,
        #[serde(rename = "outputIndex")]
        output_index: u16,
        #[serde(rename = "spendingTransaction")]
        spending_transaction: Option<Value>,
        output: Value,
    },
    /// Response of GET /api/<api_version>/transactions/<message_id>
    Transaction(Transaction),
    /// Response of GET /api/<api_version>/transactions/ed25519/<address>
    Transactions { transactions: Vec<Transaction> },
    TransactionHistory {
        address: String,
        transactions: Vec<Transfer>,
    },
    /// Response of GET /api/<api_version>/milestone/<index>
    Milestone {
        #[serde(rename = "index")]
        milestone_index: u32,
        #[serde(rename = "messageId")]
        message_id: String,
        timestamp: u64,
    },
    /// Response of GET /api/<api_version>/analytics/addresses[?start_timestamp=<u32>&end_timestamp=<u32>]
    AddressAnalytics {
        #[serde(rename = "totalAddresses")]
        total_addresses: u64,
        #[serde(rename = "receivingAddresses")]
        recv_addresses: u64,
        #[serde(rename = "sendingAddresses")]
        send_addresses: u64,
    },
}

impl IntoResponse for ListenerResponse {
    fn into_response(self) -> axum::response::Response {
        let success = SuccessBody::from(self);
        axum::Json(success).into_response()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Transfer {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "outputIndex")]
    pub output_index: u16,
    #[serde(rename = "isSpending")]
    pub is_spending: bool,
    #[serde(rename = "inclusionState")]
    pub inclusion_state: Option<LedgerInclusionState>,
    #[serde(rename = "messageId")]
    pub message_id: String,
    pub amount: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Record {
    pub id: String,
    #[serde(rename = "inclusionState")]
    pub inclusion_state: Option<LedgerInclusionState>,
    #[serde(rename = "milestoneIndex")]
    pub milestone_index: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Transaction {
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MaybeSpentOutput {
    pub output: Value,
    #[serde(rename = "spendingMessageId")]
    pub spending_message_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Unlock {
    #[serde(rename = "messageId")]
    pub message_id: String,
    pub block: Value,
}

/// A success wrapper for API responses
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SuccessBody<T> {
    data: T,
}

impl<T> Deref for SuccessBody<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> SuccessBody<T> {
    /// Create a new SuccessBody from any inner type
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> From<T> for SuccessBody<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}
