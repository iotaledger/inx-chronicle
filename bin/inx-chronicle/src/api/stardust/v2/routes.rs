// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Extension, Path},
    routing::*,
    Router,
};
use bee_message_stardust::payload::milestone::MilestoneId;
use chronicle::{
    db::MongoDb,
    types,
    stardust::{output::OutputId, payload::transaction::TransactionId, MessageId},
};
use futures::TryStreamExt;

use super::responses::*;
use crate::api::{
    error::ApiError,
    extractors::{Expanded, Pagination},
    responses::Record,
    ApiResult,
};

pub fn routes() -> Router {
    Router::new()
        .nest(
            "/messages",
            Router::new()
                .route("/:message_id", get(message))
                .route("/:message_id/raw", get(message_raw))
                .route("/:message_id/metadata", get(message_metadata))
                .route("/:message_id/children", get(message_children)),
        )
        .nest(
            "/outputs",
            Router::new()
                .route("/:output_id", get(output))
                .route("/:output_id/metadata", get(output_metadata)),
        )
        .nest(
            "/transactions",
            Router::new().route("/:transaction_id/included-message", get(transaction_included_message)),
        )
        .nest(
            "/milestones",
            Router::new()
                .route("/:milestone_id", get(milestone))
                .route("/by-index/:index", get(milestone_by_index)),
        )
}

async fn message(database: Extension<MongoDb>, Path(message_id): Path<String>) -> ApiResult<MessageResponse> {
    let message_id_dto = types::MessageId::from(MessageId::from_str(&message_id)?);
    let rec = database
        .get_message(&message_id_dto)
        .await?
        .ok_or(ApiError::NoResults)?;
    Ok(MessageResponse {
        protocol_version: rec.message.protocol_version,
        parents: rec.message.parents.iter().map(|m| m.to_hex()).collect(),
        payload: rec.message.payload,
        nonce: rec.message.nonce,
    })
}

async fn message_raw(database: Extension<MongoDb>, Path(message_id): Path<String>) -> ApiResult<Vec<u8>> {
    let message_id_dto = types::MessageId::from(MessageId::from_str(&message_id)?);
    let rec = database
        .get_message(&message_id_dto)
        .await?
        .ok_or(ApiError::NoResults)?;
    Ok(rec.raw)
}

async fn message_metadata(
    database: Extension<MongoDb>,
    Path(message_id): Path<String>,
) -> ApiResult<MessageMetadataResponse> {
    let message_id_dto = types::MessageId::from(MessageId::from_str(&message_id)?);
    let rec = database
        .get_message(&message_id_dto)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(MessageMetadataResponse {
        message_id: rec.message.id.to_hex(),
        parent_message_ids: rec.message.parents.iter().map(|id| id.to_hex()).collect(),
        is_solid: rec.metadata.as_ref().map(|d| d.is_solid),
        referenced_by_milestone_index: rec.metadata.as_ref().map(|d| d.referenced_by_milestone_index),
        milestone_index: rec.metadata.as_ref().map(|d| d.milestone_index),
        should_promote: rec.metadata.as_ref().map(|d| d.should_promote),
        should_reattach: rec.metadata.as_ref().map(|d| d.should_reattach),
        ledger_inclusion_state: rec.metadata.as_ref().map(|d| d.inclusion_state),
        conflict_reason: rec.metadata.as_ref().and_then(|d| d.conflict_reason).map(|c| c as u8),
    })
}

async fn message_children(
    database: Extension<MongoDb>,
    Path(message_id): Path<String>,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
) -> ApiResult<MessageChildrenResponse> {
    let message_id_dto = types::MessageId::from(MessageId::from_str(&message_id)?);
    let messages = database
        .get_message_children(&message_id_dto, page_size, page)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(MessageChildrenResponse {
        message_id,
        max_results: page_size,
        count: messages.len(),
        children_message_ids: messages
            .into_iter()
            .map(|rec| {
                if expanded {
                    Record {
                        id: rec.message.id.to_hex(),
                        inclusion_state: rec.metadata.as_ref().map(|d| d.inclusion_state),
                        milestone_index: rec.metadata.as_ref().map(|d| d.referenced_by_milestone_index),
                    }
                    .into()
                } else {
                    rec.message.id.to_hex().into()
                }
            })
            .collect(),
    })
}

async fn output(database: Extension<MongoDb>, Path(output_id): Path<String>) -> ApiResult<OutputResponse> {
    let output_id = types::OutputId::from(&OutputId::from_str(&output_id)?);
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
        message_id: output_res.message_id.to_hex(),
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
    let output_id = types::OutputId::from(&OutputId::from_str(&output_id)?);
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
        message_id: output_res.message_id.to_hex(),
        transaction_id: output_id.transaction_id.to_hex(),
        output_index: output_id.index,
        is_spent: spending_transaction.is_some(),
        milestone_index_spent: spending_ms_index,
        milestone_ts_spent: spending_ms
            .as_ref()
            .map(|ms| (ms.milestone_timestamp.timestamp_millis() / 1000) as u32),
        transaction_id_spent: spending_transaction.as_ref().map(|txn| {
            if let Some(types::Payload::Transaction(payload)) = &txn.message.payload {
                payload.id.to_hex()
            } else {
                unreachable!()
            }
        }),
        milestone_index_booked: booked_ms_index,
        milestone_ts_booked: (booked_ms.milestone_timestamp.timestamp_millis() / 1000) as u32,
    })
}

async fn transaction_included_message(
    database: Extension<MongoDb>,
    Path(transaction_id): Path<String>,
) -> ApiResult<MessageResponse> {
    let transaction_id_dto = types::TransactionId::from(TransactionId::from_str(&transaction_id)?);
    let rec = database
        .get_message_for_transaction(&transaction_id_dto)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(MessageResponse {
        protocol_version: rec.message.protocol_version,
        parents: rec.message.parents.iter().map(|m| m.to_hex()).collect(),
        payload: rec.message.payload,
        nonce: rec.message.nonce,
    })
}

async fn milestone(database: Extension<MongoDb>, Path(milestone_id): Path<String>) -> ApiResult<MilestoneResponse> {
    let milestone_id_dto = types::MilestoneId::from(MilestoneId::from_str(&milestone_id)?);
    database
        .get_milestone_record(&milestone_id_dto)
        .await?
        .ok_or(ApiError::NoResults)
        .map(|rec| MilestoneResponse {
            payload: types::Payload::Milestone(Box::new(rec.payload)),
        })
}

async fn milestone_by_index(database: Extension<MongoDb>, Path(index): Path<u32>) -> ApiResult<MilestoneResponse> {
    database
        .get_milestone_record_by_index(index)
        .await?
        .ok_or(ApiError::NoResults)
        .map(|rec| MilestoneResponse {
            payload: types::Payload::Milestone(Box::new(rec.payload)),
        })
}
