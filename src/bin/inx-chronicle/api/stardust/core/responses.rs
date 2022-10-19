// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::api::response as iota;
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

/// A wrapper struct that allows us to implement [`IntoResponse`](axum::response::IntoResponse) for the foreign
/// responses from [`iota_types`](iota_types::api::response).
#[derive(Clone, Debug, Serialize, derive_more::From)]
pub struct IotaResponse<T: Serialize>(T);

impl<T: Serialize> axum::response::IntoResponse for IotaResponse<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

/// A wrapper struct that allows us to implement [`IntoResponse`](axum::response::IntoResponse) for the foreign
/// raw responses from [`iota_types`](iota_types::api::response).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IotaRawResponse<T: Serialize> {
    Json(T),
    Raw(Vec<u8>),
}

impl<T: Serialize> axum::response::IntoResponse for IotaRawResponse<T> {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Json(res) => axum::Json(res).into_response(),
            Self::Raw(bytes) => bytes.into_response(),
        }
    }
}
