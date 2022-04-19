// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use axum::response::IntoResponse;
use chronicle::db::model::sync::SyncData;
use serde::{Deserialize, Serialize};

/// Response of GET /api/<api_version>/info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    #[serde(rename = "isHealthy")]
    pub is_healthy: bool,
}

impl IntoResponse for InfoResponse {
    fn into_response(self) -> axum::response::Response {
        SuccessBody::from(self).into_response()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncDataResponse(pub SyncData);

impl IntoResponse for SyncDataResponse {
    fn into_response(self) -> axum::response::Response {
        SuccessBody::from(self).into_response()
    }
}

/// A success wrapper for API responses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuccessBody<T> {
    data: T,
}

impl<T> Deref for SuccessBody<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> SuccessBody<T> {
    /// Create a new SuccessBody from any inner type
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> From<T> for SuccessBody<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<T: Serialize> IntoResponse for SuccessBody<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self).into_response()
    }
}
