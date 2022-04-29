// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::Router;
use chronicle::db::MongoDb;
use time::OffsetDateTime;

use super::{ApiError, ApiResult};

#[cfg(feature = "api-analytics")]
pub mod analytics;
#[cfg(feature = "api-explorer")]
pub mod explorer;
#[cfg(feature = "api-node")]
pub mod v2;

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new();

    #[cfg(feature = "api-analytics")]
    {
        router = router.nest("/analytics", analytics::routes());
    }

    #[cfg(feature = "api-explorer")]
    {
        router = router.nest("/explorer", explorer::routes());
    }

    #[cfg(feature = "api-node")]
    {
        router = router.nest("/v2", v2::routes());
    }

    router
}

pub(crate) async fn start_milestone(database: &MongoDb, start_timestamp: OffsetDateTime) -> ApiResult<u32> {
    database
        .find_first_milestone(start_timestamp)
        .await?
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn end_milestone(database: &MongoDb, end_timestamp: OffsetDateTime) -> ApiResult<u32> {
    database
        .find_last_milestone(end_timestamp)
        .await?
        .ok_or(ApiError::NotFound)
}
