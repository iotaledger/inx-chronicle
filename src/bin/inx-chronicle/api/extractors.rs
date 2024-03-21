// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts, Query},
    http::request::Parts,
};
use serde::Deserialize;

use super::{
    config::ApiConfigData,
    error::{ApiError, RequestError},
    DEFAULT_PAGE_SIZE,
};

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Pagination {
    pub page_size: usize,
    pub page: usize,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page_size: DEFAULT_PAGE_SIZE,
            page: 0,
        }
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Pagination
where
    Arc<ApiConfigData>: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(mut pagination) = Query::<Pagination>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        let config = Arc::<ApiConfigData>::from_ref(state);
        pagination.page_size = pagination.page_size.min(config.max_page_size);
        Ok(pagination)
    }
}

#[derive(Copy, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ListRoutesQuery {
    pub depth: Option<usize>,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for ListRoutesQuery {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<ListRoutesQuery>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        Ok(query)
    }
}

#[derive(Copy, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct TimeRangeQuery {
    start_timestamp: Option<u64>,
    end_timestamp: Option<u64>,
}

#[derive(Copy, Clone)]
pub struct TimeRange {
    pub start_timestamp: Option<u64>,
    pub end_timestamp: Option<u64>,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for TimeRange {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(TimeRangeQuery {
            start_timestamp,
            end_timestamp,
        }) = Query::<TimeRangeQuery>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        if matches!((start_timestamp, end_timestamp), (Some(start), Some(end)) if end < start) {
            return Err(ApiError::from(RequestError::BadTimeRange));
        }
        let time_range = TimeRange {
            start_timestamp: start_timestamp.map(Into::into),
            end_timestamp: end_timestamp.map(Into::into),
        };
        Ok(time_range)
    }
}

#[cfg(test)]
mod test {
    use axum::{body::Body, extract::FromRequest, http::Request};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::api::ApiConfig;

    #[tokio::test]
    async fn page_size_clamped() {
        let state = Arc::new(ApiConfigData::try_from(ApiConfig::default()).unwrap());
        let mut req = Parts::from_request(
            Request::builder()
                .method("GET")
                .uri("/?pageSize=9999999")
                .body(Body::empty())
                .unwrap(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(
            Pagination::from_request_parts(&mut req, &state).await.unwrap(),
            Pagination {
                page_size: 1000,
                ..Default::default()
            }
        );
    }
}
