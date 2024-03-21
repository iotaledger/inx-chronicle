// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains protocol types.

use iota_sdk::types::block::{protocol, slot::EpochIndex};
use serde::{Deserialize, Serialize};

/// Protocol parameters and their start epoch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ProtocolParameters {
    pub start_epoch: EpochIndex,
    pub parameters: protocol::ProtocolParameters,
}
