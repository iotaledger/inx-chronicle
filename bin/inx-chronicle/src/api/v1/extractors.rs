// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use hex::FromHex;
use serde::{Deserialize, Serialize};

use crate::api::error::APIError;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MessagesQuery {
    pub index: Option<String>,
    pub included: bool,
}

#[async_trait]
impl<B: Send> FromRequest<B> for MessagesQuery {
    type Rejection = APIError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(MessagesQuery { mut index, included }) = Query::<MessagesQuery>::from_request(req)
            .await
            .map_err(APIError::QueryError)?;

        let query = req.uri().query().unwrap_or_default();
        let query = serde_urlencoded::from_str::<HashMap<String, String>>(query).map_err(APIError::other)?;
        let utf8 = query
            .get("utf8")
            .map(|s| s.parse::<bool>())
            .transpose()
            .map_err(APIError::bad_parse)?;

        if let Some(index) = index.as_mut() {
            if let Some(true) = utf8 {
                *index = hex::encode(&*index);
            }
            let index_bytes = Vec::<u8>::from_hex(index).map_err(|_| APIError::InvalidHex)?;
            if index_bytes.len() > 64 {
                return Err(APIError::IndexTooLarge);
            }
        }

        Ok(MessagesQuery { index, included })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputsQuery {
    pub address: Option<String>,
    pub included: bool,
}

impl Default for OutputsQuery {
    fn default() -> Self {
        Self {
            address: None,
            included: true,
        }
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for OutputsQuery {
    type Rejection = APIError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<OutputsQuery>::from_request(req)
            .await
            .map_err(APIError::QueryError)?;
        Ok(query)
    }
}
