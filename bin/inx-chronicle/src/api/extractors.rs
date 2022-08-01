// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use super::{error::ApiError, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};

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
            .map_err(ApiError::QueryError)?;
        pagination.page_size = pagination.page_size.min(MAX_PAGE_SIZE);
        Ok(pagination)
    }
}

#[derive(Copy, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct TimeRangeQuery {
    start_timestamp: Option<u32>,
    end_timestamp: Option<u32>,
}

#[cfg(feature = "stardust")]
mod stardust {
    use chronicle::types::stardust::milestone::MilestoneTimestamp;

    use super::*;

    #[derive(Copy, Clone)]
    pub struct TimeRange {
        pub start_timestamp: MilestoneTimestamp,
        pub end_timestamp: MilestoneTimestamp,
    }

    fn days_ago_utc(days: i64) -> u32 {
        let then = OffsetDateTime::now_utc() - Duration::days(days);
        then.unix_timestamp() as u32
    }

    fn now_utc() -> u32 {
        OffsetDateTime::now_utc().unix_timestamp() as u32
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
                .map_err(ApiError::QueryError)?;
            let time_range = TimeRange {
                start_timestamp: start_timestamp.unwrap_or_else(|| days_ago_utc(30)).into(),
                end_timestamp: end_timestamp.unwrap_or_else(now_utc).into(),
            };
            if time_range.end_timestamp < time_range.start_timestamp {
                return Err(ApiError::BadTimeRange);
            }
            Ok(time_range)
        }
    }
}

#[cfg(feature = "stardust")]
pub use stardust::*;

#[cfg(test)]
mod test {
    use axum::{
        extract::{FromRequest, RequestParts},
        http::Request,
    };

    use super::*;

    #[tokio::test]
    async fn page_size_clamped() {
        let mut req = RequestParts::new(
            Request::builder()
                .method("GET")
                .uri("/?pageSize=9999999")
                .body(())
                .unwrap(),
        );
        assert_eq!(
            Pagination::from_request(&mut req).await.unwrap(),
            Pagination {
                page_size: MAX_PAGE_SIZE,
                ..Default::default()
            }
        );
    }
}
