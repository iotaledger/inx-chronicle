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
    dto,
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
                // TODO: For some reason, this route prevents the API from working.
                //.route("/:transaction_id/:idx", get(output_by_transaction_id)),
        )
        .nest(
            "/transactions",
            Router::new()
                .route("/:message_id", get(transaction_for_message))
                .route("/included-message/:transaction_id", get(transaction_included_message)),
        )
        .route("/milestones/:index", get(milestone))
}

async fn message(database: Extension<MongoDb>, Path(message_id): Path<String>) -> ApiResult<MessageResponse> {
    let message_id_dto = dto::MessageId::from(MessageId::from_str(&message_id)?);
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
    let message_id_dto = dto::MessageId::from(MessageId::from_str(&message_id)?);
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
    let message_id_dto = dto::MessageId::from(MessageId::from_str(&message_id)?);
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
    let message_id_dto = dto::MessageId::from(MessageId::from_str(&message_id)?);
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
    let output_id = OutputId::from_str(&output_id)?;
    output_by_transaction_id(
        database,
        Path((output_id.transaction_id().to_string(), output_id.index())),
    )
    .await
}

async fn output_by_transaction_id(
    database: Extension<MongoDb>,
    Path((transaction_id, idx)): Path<(String, u16)>,
) -> ApiResult<OutputResponse> {
    let transaction_id_dto = dto::TransactionId::from(TransactionId::from_str(&transaction_id)?);
    let output_res = database
        .get_output_by_transaction_id(&transaction_id_dto, idx)
        .await?
        .ok_or(ApiError::NoResults)?;

    let spending_transaction = database.get_spending_transaction(&transaction_id_dto, idx).await?;

    Ok(OutputResponse {
        message_id: output_res.message_id.to_hex(),
        transaction_id: transaction_id.to_string(),
        output_index: idx,
        spending_transaction: spending_transaction.map(|rec| rec.message),
        output: output_res.output,
    })
}

async fn transaction_for_message(
    database: Extension<MongoDb>,
    Path(message_id): Path<String>,
) -> ApiResult<TransactionResponse> {
    let message_id_dto = dto::MessageId::from(MessageId::from_str(&message_id)?);
    let rec = database
        .get_message(&message_id_dto)
        .await?
        .ok_or(ApiError::NoResults)?;
    if let Some(dto::Payload::Transaction(payload)) = rec.message.payload {
        let dto::TransactionEssence::Regular { inputs, outputs, .. } = payload.essence;
        Ok(TransactionResponse {
            message_id,
            milestone_index: rec.metadata.as_ref().map(|d| d.referenced_by_milestone_index),
            outputs: outputs.into(),
            inputs: inputs.into(),
        })
    } else {
        Err(ApiError::NoResults)
    }
}

async fn transaction_included_message(
    database: Extension<MongoDb>,
    Path(transaction_id): Path<String>,
) -> ApiResult<MessageResponse> {
    let transaction_id_dto = dto::TransactionId::from(TransactionId::from_str(&transaction_id)?);
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

async fn milestone(database: Extension<MongoDb>, Path(index): Path<u32>) -> ApiResult<MilestoneResponse> {
    database
        .get_milestone_record_by_index(index)
        .await?
        .ok_or(ApiError::NoResults)
        .map(|rec| MilestoneResponse {
            payload: dto::Payload::Milestone(Box::new(rec.payload)),
        })
}
