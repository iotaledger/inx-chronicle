// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use serde::Deserialize;

use crate::api::ApiError;

const MAX_TOP_RICHLIST: usize = 1000;
const DEFAULT_TOP_RICHLIST: usize = 100;

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RichlistQuery {
    pub top: usize,
}

impl Default for RichlistQuery {
    fn default() -> Self {
        Self {
            top: DEFAULT_TOP_RICHLIST,
        }
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for RichlistQuery {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut query) = Query::<RichlistQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        query.top = query.top.min(MAX_TOP_RICHLIST);
        Ok(query)
    }
}
