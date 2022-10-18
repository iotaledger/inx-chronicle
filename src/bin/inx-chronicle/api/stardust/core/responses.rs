// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::api::response::{
    self as iota, BlockResponse as IotaBlockResponse, MilestoneResponse as IotaMilestoneResponse,
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

/// Wraps responses from [`iota_types`](iota_types::api::response) so that we can implement the foreign trait
/// [`IntoResponse`](axum::response::IntoResponse).
#[macro_export]
macro_rules! impl_into_response_foreign {
    ($iota_response:ty, $own_response:ident) => {
        /// Wrapper struct around [`$iota_response`] used to implement [`IntoResponse`](axum::response::IntoResponse)
        #[derive(Clone, Debug, Serialize, Deserialize, derive_more::From)]
        pub struct $own_response($iota_response);

        impl axum::response::IntoResponse for $own_response {
            fn into_response(self) -> axum::response::Response {
                axum::Json(self.0).into_response()
            }
        }
    };
}

/// Wraps responses from [`iota_types`](iota_types::api::response) that also return raw bytes, so that we can implement
/// the foreign trait [`IntoResponse`](axum::response::IntoResponse).
#[macro_export]
macro_rules! impl_into_response_foreign_with_raw {
    ($iota_response:ident, $own_response:ident) => {
        /// Wrapper struct around [`$iota_response`] used to implement [`IntoResponse`](axum::response::IntoResponse)
        #[derive(Clone, Debug, Serialize, Deserialize, derive_more::From)]
        pub struct $own_response($iota_response);

        impl axum::response::IntoResponse for $own_response {
            fn into_response(self) -> axum::response::Response {
                match self.0 {
                    $iota_response::Json(res) => axum::Json(res).into_response(),
                    $iota_response::Raw(bytes) => bytes.into_response(),
                }
            }
        }
    };
}

impl_into_response_foreign!(iota::BlockMetadataResponse, BlockMetadataResponse);
impl_into_response_foreign!(iota::OutputMetadataResponse, OutputMetadataResponse);
impl_into_response_foreign!(iota::ReceiptsResponse, ReceiptsResponse);
impl_into_response_foreign!(iota::TreasuryResponse, TreasuryResponse);
impl_into_response_foreign!(iota::UtxoChangesResponse, UtxoChangesResponse);

impl_into_response_foreign_with_raw!(IotaBlockResponse, BlockResponse);
impl_into_response_foreign_with_raw!(IotaMilestoneResponse, MilestoneResponse);
