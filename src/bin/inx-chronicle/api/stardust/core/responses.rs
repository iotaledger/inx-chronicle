// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::{
    api::response::{self as iota},
    block::{payload::dto::MilestonePayloadDto, BlockDto},
};
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

/// Response of `GET /api/info`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    pub status: iota::StatusResponse,
    pub protocol: iota::ProtocolResponse,
}

impl_success_response!(InfoResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputResponse {
    Json(Box<iota_types::api::response::OutputResponse>),
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlockResponse {
    Json(Box<BlockDto>),
    Raw(Vec<u8>),
}

impl axum::response::IntoResponse for BlockResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            BlockResponse::Json(res) => axum::Json(res).into_response(),
            BlockResponse::Raw(bytes) => bytes.into_response(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MilestoneResponse {
    Json(MilestonePayloadDto),
    Raw(Vec<u8>),
}

impl axum::response::IntoResponse for MilestoneResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            MilestoneResponse::Json(res) => axum::Json(res).into_response(),
            MilestoneResponse::Raw(bytes) => bytes.into_response(),
        }
    }
}

/// A wrapper struct that allows us to implement [`IntoResponse`](axum::response::IntoResponse) for the foreign
/// responses from [`iota_types`](iota_types::api::response).
#[derive(Clone, Debug, Serialize, derive_more::From)]
pub struct IntoResponseWrapper<T: Serialize>(T);

impl<T: Serialize> axum::response::IntoResponse for IntoResponseWrapper<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}
