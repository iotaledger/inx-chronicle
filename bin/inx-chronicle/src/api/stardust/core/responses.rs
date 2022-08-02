// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_api_types_stardust::responses::{ProtocolResponse, StatusResponse};
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

/// Response of `GET /api/info`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    pub status: StatusResponse,
    pub protocol: ProtocolResponse,
}

impl_success_response!(InfoResponse);

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
