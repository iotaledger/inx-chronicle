// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Query},
    Extension,
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
impl<B: Send> FromRequest<B> for Pagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut pagination) = Query::<Pagination>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;
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
impl<B: Send> FromRequest<B> for ListRoutesQuery {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<ListRoutesQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        Ok(query)
    }
}

#[derive(Copy, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct TimeRangeQuery {
    start_timestamp: Option<u32>,
    end_timestamp: Option<u32>,
}

mod stardust {
    use chronicle::types::stardust::milestone::MilestoneTimestamp;

    use super::*;
    use crate::api::error::RequestError;

    #[derive(Copy, Clone)]
    pub struct TimeRange {
        pub start_timestamp: Option<MilestoneTimestamp>,
        pub end_timestamp: Option<MilestoneTimestamp>,
    }

    #[async_trait]
    impl<B: Send> FromRequest<B> for TimeRange {
        type Rejection = ApiError;

        async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
            let Query(TimeRangeQuery {
                start_timestamp,
                end_timestamp,
            }) = Query::<TimeRangeQuery>::from_request(req)
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
}

pub use stardust::*;

#[cfg(test)]
mod test {
    use axum::{
        extract::{FromRequest, RequestParts},
        http::Request,
    };

    use super::*;
    use crate::api::ApiConfig;

    #[tokio::test]
    async fn page_size_clamped() {
        let mut req = RequestParts::new(
            Request::builder()
                .method("GET")
                .uri("/?pageSize=9999999")
                .extension(ApiConfigData::try_from(ApiConfig::default()).unwrap())
                .body(())
                .unwrap(),
        );
        assert_eq!(
            Pagination::from_request(&mut req).await.unwrap(),
            Pagination {
                page_size: 1000,
                ..Default::default()
            }
        );
    }
}
