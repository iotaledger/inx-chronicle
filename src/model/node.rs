// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::slot::{EpochIndex, SlotCommitmentId, SlotIndex};
use serde::{Deserialize, Serialize};

use super::protocol::ProtocolParameters;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseToken {
    pub name: String,
    pub ticker_symbol: String,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subunit: Option<String>,
    pub decimals: u32,
    pub use_metric_prefix: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfiguration {
    pub base_token: BaseToken,
    pub protocol_parameters: Vec<ProtocolParameters>,
}

impl NodeConfiguration {
    pub fn latest_parameters(&self) -> &iota_sdk::types::block::protocol::ProtocolParameters {
        &self.protocol_parameters.last().unwrap().parameters
    }
}

pub struct NodeStatus {
    pub is_healthy: bool,
    pub accepted_tangle_time: Option<u64>,
    pub relative_accepted_tangle_time: Option<u64>,
    pub confirmed_tangle_time: Option<u64>,
    pub relative_confirmed_tangle_time: Option<u64>,
    pub latest_commitment_id: SlotCommitmentId,
    pub latest_finalized_slot: SlotIndex,
    pub latest_accepted_block_slot: Option<SlotIndex>,
    pub latest_confirmed_block_slot: Option<SlotIndex>,
    pub pruning_epoch: EpochIndex,
}
