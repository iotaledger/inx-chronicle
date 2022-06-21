// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

/// Response of GET /api/core/v2/blocks/{block_id}/children.
/// Returns all children of a specific block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockChildrenResponse {
    #[serde(rename = "blockId")]
    pub block_id: String,
    #[serde(rename = "maxResults")]
    pub max_results: usize,
    pub count: usize,
    pub children: Vec<String>,
}

impl_success_response!(BlockChildrenResponse);
