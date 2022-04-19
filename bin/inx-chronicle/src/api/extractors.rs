// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use super::error::APIError;

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
    type Rejection = APIError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(pagination) = Query::<Pagination>::from_request(req)
            .await
            .map_err(APIError::QueryError)?;
        Ok(pagination)
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
    type Rejection = APIError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(TimeRangeQuery {
            start_timestamp,
            end_timestamp,
        }) = Query::<TimeRangeQuery>::from_request(req)
            .await
            .map_err(APIError::QueryError)?;
        let time_range = TimeRange {
            start_timestamp: start_timestamp
                .map(|t| OffsetDateTime::from_unix_timestamp(t as i64))
                .transpose()
                .map_err(APIError::bad_parse)?
                .unwrap_or_else(|| OffsetDateTime::now_utc() - Duration::days(30)),
            end_timestamp: end_timestamp
                .map(|t| OffsetDateTime::from_unix_timestamp(t as i64))
                .transpose()
                .map_err(APIError::bad_parse)?
                .unwrap_or_else(OffsetDateTime::now_utc),
        };
        if time_range.end_timestamp < time_range.start_timestamp {
            return Err(APIError::BadTimeRange);
        }
        Ok(time_range)
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
    type Rejection = APIError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(included) = Query::<Included>::from_request(req)
            .await
            .map_err(APIError::QueryError)?;
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
    type Rejection = APIError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(expanded) = Query::<Expanded>::from_request(req)
            .await
            .map_err(APIError::QueryError)?;
        Ok(expanded)
    }
}
