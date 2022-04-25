// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::Router;
use chronicle::{
    bson::DocExt,
    db::{model::stardust::milestone::MilestoneRecord, MongoDb},
};
use mongodb::{
    bson::{doc, DateTime},
    options::FindOptions,
};
use time::OffsetDateTime;
use tokio_stream::StreamExt;

use super::{ApiError, ApiResult};

#[cfg(feature = "api-analytics")]
pub mod analytics;
#[cfg(feature = "api-explorer")]
pub mod explorer;
#[cfg(feature = "api-indexer")]
pub mod indexer;
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

    #[cfg(feature = "api-indexer")]
    {
        router = router.nest("/indexer", indexer::routes());
    }

    #[cfg(feature = "api-node")]
    {
        router = router.nest("/v2", v2::routes());
    }

    router
}

pub(crate) async fn start_milestone(database: &MongoDb, start_timestamp: OffsetDateTime) -> ApiResult<u32> {
    database
        .doc_collection::<MilestoneRecord>()
        .find(
            doc! {"milestone_timestamp": { "$gte": DateTime::from_millis(start_timestamp.unix_timestamp() * 1000) }},
            FindOptions::builder()
                .sort(doc! {"milestone_index": 1})
                .limit(1)
                .build(),
        )
        .await?
        .try_next()
        .await?
        .map(|d| d.get_as_u32("milestone_index"))
        .transpose()?
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn end_milestone(database: &MongoDb, end_timestamp: OffsetDateTime) -> ApiResult<u32> {
    database
        .doc_collection::<MilestoneRecord>()
        .find(
            doc! {"milestone_timestamp": { "$lte": DateTime::from_millis(end_timestamp.unix_timestamp() * 1000) }},
            FindOptions::builder()
                .sort(doc! {"milestone_index": -1})
                .limit(1)
                .build(),
        )
        .await?
        .try_next()
        .await?
        .map(|d| d.get_as_u32("milestone_index"))
        .transpose()?
        .ok_or(ApiError::NotFound)
}
