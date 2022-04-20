// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use hex::FromHex;
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MessagesQuery {
    pub tag: Option<String>,
    pub included: bool,
}

impl Default for MessagesQuery {
    fn default() -> Self {
        Self {
            tag: None,
            included: true,
        }
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for MessagesQuery {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(MessagesQuery { mut tag, included }) = Query::<MessagesQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        let query = req.uri().query().unwrap_or_default();
        let query = serde_urlencoded::from_str::<HashMap<String, String>>(query).map_err(ApiError::other)?;
        let utf8 = query
            .get("utf8")
            .map(|s| s.parse::<bool>())
            .transpose()
            .map_err(ApiError::bad_parse)?;

        if let Some(tag) = tag.as_mut() {
            if let Some(true) = utf8 {
                *tag = hex::encode(&*tag);
            }
            let tag_bytes = Vec::<u8>::from_hex(tag).map_err(|_| ApiError::InvalidHex)?;
            if tag_bytes.len() > 64 {
                return Err(ApiError::TagTooLarge);
            }
        }

        Ok(MessagesQuery { tag, included })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputsQuery {
    pub address: Option<String>,
    #[serde(rename = "requiresDustReturn")]
    pub requires_dust_return: bool,
    pub sender: Option<String>,
    pub tag: Option<String>,
    pub included: bool,
}

impl Default for OutputsQuery {
    fn default() -> Self {
        Self {
            address: None,
            requires_dust_return: false,
            sender: None,
            tag: None,
            included: true,
        }
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for OutputsQuery {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<OutputsQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        Ok(query)
    }
}
