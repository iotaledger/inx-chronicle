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

/// Response of `GET /api/outputs/:outputId`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum OutputResponse {
    Json(Box<bee_api_types_stardust::responses::OutputResponse>),
    Raw(Vec<u8>),
}

impl_success_response!(OutputResponse);
