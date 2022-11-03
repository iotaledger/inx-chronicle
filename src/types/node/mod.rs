// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the node models.

use serde::{Deserialize, Serialize};

/// The [`NodeConfiguration`] type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct NodeConfiguration {
    pub base_token: BaseToken,
}

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
