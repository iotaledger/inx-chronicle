// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    str::FromStr,
};

use async_trait::async_trait;
use axum::extract::{
    FromRequest,
    Path,
    Query,
};
use chrono::{
    Duration,
    NaiveDateTime,
};
use hex::FromHex;
use serde::Deserialize;

use super::error::ListenerError;
use crate::types::message::MessageId;

#[async_trait]
impl<B: Send> FromRequest<B> for MessageId {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Path(message_id) = Path::<String>::from_request(req)
            .await
            .map_err(|e| ListenerError::PathError(e))?;
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
            .map_err(|e| ListenerError::QueryError(e))?;
        Ok(pagination)
    }
}

#[derive(Clone, Deserialize)]
pub struct Index {
    pub index: String,
}

#[async_trait]
impl<B: Send> FromRequest<B> for Index {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut index) = Query::<Index>::from_request(req)
            .await
            .map_err(|e| ListenerError::QueryError(e))?;
        let query = req.uri().query().unwrap_or_default();
        let query =
            serde_urlencoded::from_str::<HashMap<String, String>>(query).map_err(|e| ListenerError::Other(e.into()))?;
        let utf8 = query.get("utf8").map(|s| s.as_str());
        if let Some("true") = utf8 {
            index.index = hex::encode(index.index);
        }
        let index_bytes = Vec::<u8>::from_hex(&index.index).map_err(|_| ListenerError::InvalidHex)?;
        if index_bytes.len() > 64 {
            return Err(ListenerError::IndexTooLarge);
        }
        Ok(index)
    }
}

#[derive(Clone, Deserialize)]
pub struct Tag {
    pub tag: String,
}

#[async_trait]
impl<B: Send> FromRequest<B> for Tag {
    type Rejection = ListenerError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut tag) = Query::<Tag>::from_request(req)
            .await
            .map_err(|e| ListenerError::QueryError(e))?;
        let query = req.uri().query().unwrap_or_default();
        let query =
            serde_urlencoded::from_str::<HashMap<String, String>>(query).map_err(|e| ListenerError::Other(e.into()))?;
        let utf8 = query.get("utf8").map(|s| s.as_str());
        if let Some("true") = utf8 {
            tag.tag = hex::encode(tag.tag);
        }
        let tag_bytes = Vec::<u8>::from_hex(&tag.tag).map_err(|_| ListenerError::InvalidHex)?;
        if tag_bytes.len() > 64 {
            return Err(ListenerError::IndexTooLarge);
        }
        Ok(tag)
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
    pub start_timestamp: NaiveDateTime,
    pub end_timestamp: NaiveDateTime,
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
            .map_err(|e| ListenerError::QueryError(e))?;
        let time_range = TimeRange {
            start_timestamp: start_timestamp
                .map(|t| NaiveDateTime::from_timestamp(t as i64, 0))
                .unwrap_or(chrono::Utc::now().naive_utc() - Duration::days(30)),
            end_timestamp: end_timestamp
                .map(|t| NaiveDateTime::from_timestamp(t as i64, 0))
                .unwrap_or(chrono::Utc::now().naive_utc()),
        };
        if end_timestamp < start_timestamp {
            return Err(ListenerError::BadTimeRange);
        }
        Ok(time_range)
    }
}
