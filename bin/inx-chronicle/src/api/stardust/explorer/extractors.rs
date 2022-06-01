// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use serde::Deserialize;

use crate::api::ApiError;

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct HistoryPagination {
    pub page_size: usize,
    pub start_milestone_index: Option<u32>,
    pub start_output_id: Option<String>,
}

impl Default for HistoryPagination {
    fn default() -> Self {
        Self {
            page_size: 100,
            start_milestone_index: None,
            start_output_id: None,
        }
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for HistoryPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(pagination) = Query::<HistoryPagination>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        Ok(pagination)
    }
}
