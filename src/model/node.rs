// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains node related types.

use iota_sdk::types::block::slot::{EpochIndex, SlotIndex};
use serde::{Deserialize, Serialize};

use super::{protocol::ProtocolParameters, slot::Commitment};

/// Node base token configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseToken {
    /// The name of the base token.
    pub name: String,
    /// The symbol used to represent the token.
    pub ticker_symbol: String,
    /// The name of a single unit of the token.
    pub unit: String,
    /// The name of a sub-unit of the token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subunit: Option<String>,
    /// The number of allowed decimal places.
    pub decimals: u32,
}

/// Node configuation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct NodeConfiguration {
    pub base_token: BaseToken,
    /// A map of protocol parameters and start epochs.
    pub protocol_parameters: Vec<ProtocolParameters>,
}

impl NodeConfiguration {
    /// Get the latest protocol parameters.
    pub fn latest_parameters(&self) -> &iota_sdk::types::block::protocol::ProtocolParameters {
        &self.protocol_parameters.last().unwrap().parameters
    }
}

/// Status data of a node.
#[allow(missing_docs)]
pub struct NodeStatus {
    pub is_healthy: bool,
    pub last_accepted_block_slot: SlotIndex,
    pub last_confirmed_block_slot: SlotIndex,
    pub latest_commitment: Commitment,
    pub latest_finalized_commitment: Commitment,
    pub pruning_epoch: EpochIndex,
    pub is_bootstrapped: bool,
}
