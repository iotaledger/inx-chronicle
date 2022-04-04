// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use async_trait::async_trait;
use axum::extract::{
    FromRequest,
    Query,
};
use hex::FromHex;
use serde::{
    Deserialize,
    Serialize,
};
use time::{
    Duration,
    OffsetDateTime,
};

use super::error::ListenerError;

#[derive(Copy, Clone, Deserialize)]
pub enum APIVersion {
    #[serde(rename = "v1")]
    V1,
    #[serde(rename = "v2")]
    V2,
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
            .map_err(ListenerError::QueryError)?;
        Ok(pagination)
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MessagesQuery {
    pub index: Option<String>,
    pub tag: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for MessagesQuery {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(MessagesQuery { mut index, mut tag }) = Query::<MessagesQuery>::from_request(req)
            .await
            .map_err(ListenerError::QueryError)?;
        if index.is_some() || tag.is_some() {
            let query = req.uri().query().unwrap_or_default();
            let query = serde_urlencoded::from_str::<HashMap<String, String>>(query).map_err(ListenerError::other)?;
            let utf8 = query
                .get("utf8")
                .map(|s| s.parse::<bool>())
                .transpose()
                .map_err(ListenerError::bad_parse)?;

            if let Some(index) = index.as_mut() {
                if let Some(true) = utf8 {
                    *index = hex::encode(&*index);
                }
                let index_bytes = Vec::<u8>::from_hex(index).map_err(|_| ListenerError::InvalidHex)?;
                if index_bytes.len() > 64 {
                    return Err(ListenerError::IndexTooLarge);
                }
            }
            if let Some(tag) = tag.as_mut() {
                if let Some(true) = utf8 {
                    *tag = hex::encode(&*tag);
                }
                let tag_bytes = Vec::<u8>::from_hex(tag).map_err(|_| ListenerError::InvalidHex)?;
                if tag_bytes.len() > 64 {
                    return Err(ListenerError::TagTooLarge);
                }
            }
        }
        Ok(MessagesQuery { index, tag })
    }
}

#[derive(Copy, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TimeRangeQuery {
    start_timestamp: Option<u64>,
    end_timestamp: Option<u64>,
}

#[derive(Copy, Clone)]
pub struct TimeRange {
    pub start_timestamp: OffsetDateTime,
    pub end_timestamp: OffsetDateTime,
}

#[async_trait]
impl<B: Send> FromRequest<B> for TimeRange {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(TimeRangeQuery {
            start_timestamp,
            end_timestamp,
        }) = Query::<TimeRangeQuery>::from_request(req)
            .await
            .map_err(ListenerError::QueryError)?;
        let time_range = TimeRange {
            start_timestamp: start_timestamp
                .map(|t| OffsetDateTime::from_unix_timestamp(t as i64))
                .transpose()
                .map_err(ListenerError::bad_parse)?
                .unwrap_or_else(|| OffsetDateTime::now_utc() - Duration::days(30)),
            end_timestamp: end_timestamp
                .map(|t| OffsetDateTime::from_unix_timestamp(t as i64))
                .transpose()
                .map_err(ListenerError::bad_parse)?
                .unwrap_or_else(OffsetDateTime::now_utc),
        };
        if time_range.end_timestamp < time_range.start_timestamp {
            return Err(ListenerError::BadTimeRange);
        }
        Ok(time_range)
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
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<OutputsQuery>::from_request(req)
            .await
            .map_err(ListenerError::QueryError)?;
        Ok(query)
    }
}

#[derive(Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Included {
    pub included: bool,
}

impl Default for Included {
    fn default() -> Self {
        Self { included: true }
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for Included {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(included) = Query::<Included>::from_request(req)
            .await
            .map_err(ListenerError::QueryError)?;
        Ok(included)
    }
}

#[derive(Copy, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Expanded {
    pub expanded: bool,
}

#[async_trait]
impl<B: Send> FromRequest<B> for Expanded {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(expanded) = Query::<Expanded>::from_request(req)
            .await
            .map_err(ListenerError::QueryError)?;
        Ok(expanded)
    }
}
