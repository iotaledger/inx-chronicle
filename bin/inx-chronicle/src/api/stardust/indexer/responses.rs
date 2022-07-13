// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BasicOutputsResponse {
    pub ledger_index: u32,
    pub items: Vec<String>,
    pub cursor: Option<String>,
}

impl_success_response!(BasicOutputsResponse);
