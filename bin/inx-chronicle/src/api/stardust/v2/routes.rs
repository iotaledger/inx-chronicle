// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Extension, Path},
    routing::*,
    Router,
};
use chronicle::{
    db::MongoDb,
    types::{
        stardust::block::{BlockId, MilestoneId, OutputId, TransactionId},
        tangle::MilestoneIndex,
    },
};
use futures::TryStreamExt;

use super::responses::*;
use crate::api::{error::ApiError, extractors::Pagination, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .nest(
            "/blocks",
            Router::new()
                .route("/:block_id", get(block))
                .route("/:block_id/raw", get(block_raw))
                .route("/:block_id/metadata", get(block_metadata))
                .route("/:block_id/children", get(block_children)),
        )
        .nest(
            "/outputs",
            Router::new()
                .route("/:output_id", get(output))
                .route("/:output_id/metadata", get(output_metadata)),
        )
        .nest(
            "/transactions",
            Router::new().route("/:transaction_id/included-block", get(transaction_included_block)),
        )
        .nest(
            "/milestones",
            Router::new()
                .route("/:milestone_id", get(milestone))
                .route("/by-index/:index", get(milestone_by_index)),
        )
}

async fn block(database: Extension<MongoDb>, Path(block_id): Path<String>) -> ApiResult<BlockResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let rec = database.get_block(&block_id).await?.ok_or(ApiError::NoResults)?;
    Ok(BlockResponse {
        protocol_version: rec.protocol_version,
        parents: rec.parents.iter().map(|m| m.to_hex()).collect(),
        payload: rec.payload,
        nonce: rec.nonce,
    })
}

async fn block_raw(database: Extension<MongoDb>, Path(block_id): Path<String>) -> ApiResult<Vec<u8>> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    database.get_block_raw(&block_id).await?.ok_or(ApiError::NoResults)
}

async fn block_metadata(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
) -> ApiResult<BlockMetadataResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let metadata = database
        .get_block_metadata(&block_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(BlockMetadataResponse {
        block_id: metadata.block_id.to_hex(),
        parents: metadata.parents.iter().map(|id| id.to_hex()).collect(),
        is_solid: Some(metadata.is_solid),
        referenced_by_milestone_index: Some(metadata.referenced_by_milestone_index),
        milestone_index: Some(metadata.milestone_index),
        should_promote: Some(metadata.should_promote),
        should_reattach: Some(metadata.should_reattach),
        ledger_inclusion_state: Some(metadata.inclusion_state),
        conflict_reason: Some(metadata.conflict_reason as u8),
    })
}

async fn block_children(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<BlockChildrenResponse> {
    let block_id_dto = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let blocks = database
        .get_block_children(&block_id_dto, page_size, page)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(BlockChildrenResponse {
        block_id,
        max_results: page_size,
        count: blocks.len(),
        children: blocks,
    })
}

async fn output(database: Extension<MongoDb>, Path(output_id): Path<String>) -> ApiResult<OutputResponse> {
    let output_id = OutputId::from_str(&output_id).map_err(ApiError::bad_parse)?;
    let (output, metadata) = database
        .get_output_and_metadata(&output_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(OutputResponse {
        block_id: metadata.block_id.to_hex(),
        transaction_id: metadata.transaction_id.to_hex(),
        output_index: metadata.output_id.index,
        is_spent: metadata.spent.is_some(),
        milestone_index_spent: metadata.spent.as_ref().map(|s| s.spent.milestone_index),
        milestone_ts_spent: metadata.spent.as_ref().map(|s| s.spent.milestone_timestamp),
        milestone_index_booked: metadata.booked.milestone_index,
        milestone_ts_booked: metadata.booked.milestone_timestamp,
        output,
    })
}

async fn output_metadata(
    database: Extension<MongoDb>,
    Path(output_id): Path<String>,
) -> ApiResult<OutputMetadataResponse> {
    let output_id = OutputId::from_str(&output_id).map_err(ApiError::bad_parse)?;
    let metadata = database
        .get_output_metadata(&output_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(OutputMetadataResponse {
        block_id: metadata.block_id.to_hex(),
        transaction_id: metadata.transaction_id.to_hex(),
        output_index: metadata.output_id.index,
        is_spent: metadata.spent.is_some(),
        milestone_index_spent: metadata.spent.as_ref().map(|s| s.spent.milestone_index),
        milestone_ts_spent: metadata.spent.as_ref().map(|s| s.spent.milestone_timestamp),
        transaction_id_spent: metadata.spent.as_ref().map(|s| s.transaction_id.to_hex()),
        milestone_index_booked: metadata.booked.milestone_index,
        milestone_ts_booked: metadata.booked.milestone_timestamp,
    })
}

async fn transaction_included_block(
    database: Extension<MongoDb>,
    Path(transaction_id): Path<String>,
) -> ApiResult<BlockResponse> {
    let transaction_id_dto = TransactionId::from_str(&transaction_id).map_err(ApiError::bad_parse)?;
    let block = database
        .get_block_for_transaction(&transaction_id_dto)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(BlockResponse {
        protocol_version: block.protocol_version,
        parents: block.parents.iter().map(|m| m.to_hex()).collect(),
        payload: block.payload,
        nonce: block.nonce,
    })
}

async fn milestone(database: Extension<MongoDb>, Path(milestone_id): Path<String>) -> ApiResult<MilestoneResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(ApiError::bad_parse)?;
    let payload = database
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(MilestoneResponse { payload })
}

async fn milestone_by_index(
    database: Extension<MongoDb>,
    Path(index): Path<MilestoneIndex>,
) -> ApiResult<MilestoneResponse> {
    let payload = database
        .get_milestone_payload(index)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(MilestoneResponse { payload })
}
