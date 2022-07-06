// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

/// Response of GET /api/core/v2/blocks/{block_id}/children.
/// Returns all children of a specific block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockChildrenResponse {
    pub block_id: String,
    pub max_results: usize,
    pub count: usize,
    pub children: Vec<String>,
}

impl_success_response!(BlockChildrenResponse);
