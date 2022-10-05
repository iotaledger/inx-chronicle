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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputResponse {
    Json(Box<bee_api_types_stardust::responses::OutputResponse>),
    Raw(Vec<u8>),
}

impl axum::response::IntoResponse for OutputResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            OutputResponse::Json(res) => axum::Json(res).into_response(),
            OutputResponse::Raw(bytes) => bytes.into_response(),
        }
    }
}
