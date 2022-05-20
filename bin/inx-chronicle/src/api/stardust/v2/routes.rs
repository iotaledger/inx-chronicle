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
    types::stardust::block::{BlockId, MilestoneId, OutputId, Payload, TransactionId},
};
use futures::TryStreamExt;

use super::responses::*;
use crate::api::{
    error::{ApiError, ParseError},
    extractors::{Expanded, Pagination},
    responses::Record,
    ApiResult,
};

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
    let block_id_dto = BlockId::from_str(&block_id).map_err(ParseError::StorageType)?;
    let rec = database.get_block(&block_id_dto).await?.ok_or(ApiError::NoResults)?;
    Ok(BlockResponse {
        protocol_version: rec.inner.protocol_version,
        parents: rec.inner.parents.iter().map(|m| m.to_hex()).collect(),
        payload: rec.inner.payload,
        nonce: rec.inner.nonce,
    })
}

async fn block_raw(database: Extension<MongoDb>, Path(block_id): Path<String>) -> ApiResult<Vec<u8>> {
    let block_id_dto = BlockId::from_str(&block_id).map_err(ParseError::StorageType)?;
    let rec = database.get_block(&block_id_dto).await?.ok_or(ApiError::NoResults)?;
    Ok(rec.raw)
}

async fn block_metadata(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
) -> ApiResult<BlockMetadataResponse> {
    let block_id_dto = BlockId::from_str(&block_id).map_err(ParseError::StorageType)?;
    let rec = database.get_block(&block_id_dto).await?.ok_or(ApiError::NoResults)?;

    Ok(BlockMetadataResponse {
        block_id: rec.inner.block_id.to_hex(),
        parents: rec.inner.parents.iter().map(|id| id.to_hex()).collect(),
        is_solid: rec.metadata.as_ref().map(|d| d.is_solid),
        referenced_by_milestone_index: rec.metadata.as_ref().map(|d| d.referenced_by_milestone_index),
        milestone_index: rec.metadata.as_ref().map(|d| d.milestone_index),
        should_promote: rec.metadata.as_ref().map(|d| d.should_promote),
        should_reattach: rec.metadata.as_ref().map(|d| d.should_reattach),
        ledger_inclusion_state: rec.metadata.as_ref().map(|d| d.inclusion_state),
        conflict_reason: rec.metadata.as_ref().map(|d| d.conflict_reason as u8),
    })
}

async fn block_children(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
) -> ApiResult<BlockChildrenResponse> {
    let block_id_dto = BlockId::from_str(&block_id).map_err(ParseError::StorageType)?;
    let blocks = database
        .get_block_children(&block_id_dto, page_size, page)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(BlockChildrenResponse {
        block_id,
        max_results: page_size,
        count: blocks.len(),
        children: blocks
            .into_iter()
            .map(|rec| {
                if expanded {
                    Record {
                        id: rec.inner.block_id.to_hex(),
                        inclusion_state: rec.metadata.as_ref().map(|d| d.inclusion_state),
                        milestone_index: rec.metadata.as_ref().map(|d| d.referenced_by_milestone_index),
                    }
                    .into()
                } else {
                    rec.inner.block_id.to_hex().into()
                }
            })
            .collect(),
    })
}

async fn output(database: Extension<MongoDb>, Path(output_id): Path<String>) -> ApiResult<OutputResponse> {
    let output_id = OutputId::from_str(&output_id).map_err(ParseError::StorageType)?;
    let output_res = database
        .get_output(&output_id.transaction_id, output_id.index)
        .await?
        .ok_or(ApiError::NoResults)?;

    let booked_ms_index = output_res
        .metadata
        .map(|d| d.referenced_by_milestone_index)
        .ok_or(ApiError::NoResults)?;
    let booked_ms = database
        .get_milestone_record_by_index(booked_ms_index)
        .await?
        .ok_or(ApiError::NoResults)?;
    let spending_transaction = database
        .get_spending_transaction(&output_id.transaction_id, output_id.index)
        .await?;

    let spending_ms_index = spending_transaction
        .as_ref()
        .and_then(|txn| txn.metadata.as_ref().map(|d| d.referenced_by_milestone_index));
    let spending_ms = if let Some(ms_index) = spending_ms_index {
        database.get_milestone_record_by_index(ms_index).await?
    } else {
        None
    };

    Ok(OutputResponse {
        block_id: output_res.block_id.to_hex(),
        transaction_id: output_id.transaction_id.to_hex(),
        output_index: output_id.index,
        is_spent: spending_transaction.is_some(),
        milestone_index_spent: spending_ms_index,
        milestone_ts_spent: spending_ms
            .as_ref()
            .map(|ms| (ms.milestone_timestamp.timestamp_millis() / 1000) as u32),
        milestone_index_booked: booked_ms_index,
        milestone_ts_booked: (booked_ms.milestone_timestamp.timestamp_millis() / 1000) as u32,
        output: output_res.output,
    })
}

async fn output_metadata(
    database: Extension<MongoDb>,
    Path(output_id): Path<String>,
) -> ApiResult<OutputMetadataResponse> {
    let output_id = OutputId::from_str(&output_id).map_err(ParseError::StorageType)?;
    let output_res = database
        .get_output(&output_id.transaction_id, output_id.index)
        .await?
        .ok_or(ApiError::NoResults)?;

    let booked_ms_index = output_res
        .metadata
        .map(|d| d.referenced_by_milestone_index)
        .ok_or(ApiError::NoResults)?;
    let booked_ms = database
        .get_milestone_record_by_index(booked_ms_index)
        .await?
        .ok_or(ApiError::NoResults)?;
    let spending_transaction = database
        .get_spending_transaction(&output_id.transaction_id, output_id.index)
        .await?;

    let spending_ms_index = spending_transaction
        .as_ref()
        .and_then(|txn| txn.metadata.as_ref().map(|d| d.referenced_by_milestone_index));
    let spending_ms = if let Some(ms_index) = spending_ms_index {
        database.get_milestone_record_by_index(ms_index).await?
    } else {
        None
    };

    Ok(OutputMetadataResponse {
        block_id: output_res.block_id.to_hex(),
        transaction_id: output_id.transaction_id.to_hex(),
        output_index: output_id.index,
        is_spent: spending_transaction.is_some(),
        milestone_index_spent: spending_ms_index,
        milestone_ts_spent: spending_ms
            .as_ref()
            .map(|ms| (ms.milestone_timestamp.timestamp_millis() / 1000) as u32),
        transaction_id_spent: spending_transaction.as_ref().map(|txn| {
            if let Some(Payload::Transaction(payload)) = &txn.inner.payload {
                payload.id.to_hex()
            } else {
                unreachable!()
            }
        }),
        milestone_index_booked: booked_ms_index,
        milestone_ts_booked: (booked_ms.milestone_timestamp.timestamp_millis() / 1000) as u32,
    })
}

async fn transaction_included_block(
    database: Extension<MongoDb>,
    Path(transaction_id): Path<String>,
) -> ApiResult<BlockResponse> {
    let transaction_id_dto = TransactionId::from_str(&transaction_id).map_err(ParseError::StorageType)?;
    let rec = database
        .get_block_for_transaction(&transaction_id_dto)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(BlockResponse {
        protocol_version: rec.inner.protocol_version,
        parents: rec.inner.parents.iter().map(|m| m.to_hex()).collect(),
        payload: rec.inner.payload,
        nonce: rec.inner.nonce,
    })
}

async fn milestone(database: Extension<MongoDb>, Path(milestone_id): Path<String>) -> ApiResult<MilestoneResponse> {
    let milestone_id_dto = MilestoneId::from_str(&milestone_id).map_err(ParseError::StorageType)?;
    database
        .get_milestone_record(&milestone_id_dto)
        .await?
        .ok_or(ApiError::NoResults)
        .map(|rec| MilestoneResponse {
            payload: Payload::Milestone(Box::new(rec.payload)),
        })
}

async fn milestone_by_index(database: Extension<MongoDb>, Path(index): Path<u32>) -> ApiResult<MilestoneResponse> {
    database
        .get_milestone_record_by_index(index)
        .await?
        .ok_or(ApiError::NoResults)
        .map(|rec| MilestoneResponse {
            payload: Payload::Milestone(Box::new(rec.payload)),
        })
}
