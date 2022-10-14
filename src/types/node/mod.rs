// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the node models.

use serde::{Deserialize, Serialize};

/// The [`BaseToken`] type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BaseToken {
    pub name: String,
    pub ticker_symbol: String,
    pub unit: String,
    pub subunit: String,
    pub decimals: u32,
    pub use_metric_prefix: bool,
}

/// The [`NodeConfiguration`] type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct NodeConfiguration {
    pub milestone_public_key_count: u32,
    pub base_token: BaseToken,
    pub supported_protocol_versions: Box<[u8]>,
}

impl From<bee_inx::NodeConfiguration> for NodeConfiguration {
    fn from(value: bee_inx::NodeConfiguration) -> Self {
        Self {
            milestone_public_key_count: value.milestone_public_key_count,
            base_token: BaseToken {
                name: value.base_token.name,
                ticker_symbol: value.base_token.ticker_symbol,
                unit: value.base_token.unit,
                subunit: value.base_token.subunit,
                decimals: value.base_token.decimals,
                use_metric_prefix: value.base_token.use_metric_prefix,
            },
            supported_protocol_versions: value.supported_protocol_versions,
        }
    }
}
