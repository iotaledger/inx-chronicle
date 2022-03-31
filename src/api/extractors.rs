// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use async_trait::async_trait;
use axum::extract::{
    FromRequest,
    Path,
    Query,
};
use serde::Deserialize;

use super::error::ListenerError;
use crate::types::message::MessageId;

#[async_trait]
impl<B: Send> FromRequest<B> for MessageId {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Path(message_id) = Path::<String>::from_request(req)
            .await
            .map_err(|e| ListenerError::PathError(e.into()))?;
        Ok(MessageId::from_str(&message_id).map_err(|e| ListenerError::BadParse(e.into()))?)
    }
}

#[derive(Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Pagination {
    pub page_size: usize,
    pub page: usize,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page_size: 100,
            page: 0,
        }
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for Pagination {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(pagination) = Query::<Pagination>::from_request(req)
            .await
            .map_err(|e| ListenerError::QueryError(e.into()))?;
        Ok(pagination)
    }
}
