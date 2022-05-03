// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Extension, Path},
    routing::*,
    Router,
};
use chronicle::{
    db::{
        bson::{BsonExt, DocExt, U64},
        MongoDb,
    },
    stardust::{output::OutputId, payload::milestone::MilestoneId, MessageId},
};
use futures::TryStreamExt;
use mongodb::bson::from_document;

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
    let mut rec = database
        .get_message(&MessageId::from_str(&message_id)?)
        .await?
        .ok_or(ApiError::NoResults)?;
    let mut message = rec.take_document("message")?;
    Ok(MessageResponse {
        protocol_version: message.get_as_u8("protocol_version")?,
        parents: message
            .take_array("parents")?
            .iter()
            .map(|m| m.as_string())
            .collect::<Result<_, _>>()?,
        payload: message.take_bson("payload").ok().map(Into::into),
        nonce: from_document::<U64>(message.take_document("nonce")?)?.into(),
    })
}

async fn message_raw(database: Extension<MongoDb>, Path(message_id): Path<String>) -> ApiResult<Vec<u8>> {
    let mut rec = database
        .get_message(&MessageId::from_str(&message_id)?)
        .await?
        .ok_or(ApiError::NoResults)?;
    let mut message = rec.take_document("message")?;
    Ok(message.take_bytes("raw")?)
}

async fn message_metadata(
    database: Extension<MongoDb>,
    Path(message_id): Path<String>,
) -> ApiResult<MessageMetadataResponse> {
    let mut rec = database
        .get_message(&MessageId::from_str(&message_id)?)
        .await?
        .ok_or(ApiError::NoResults)?;
    let mut message = rec.take_document("message")?;
    let metadata = rec.take_document("metadata").ok();

    Ok(MessageMetadataResponse {
        message_id: rec.get_as_string("message_id")?,
        parent_message_ids: message
            .take_array("parents")?
            .iter()
            .map(|id| id.as_string())
            .collect::<Result<_, _>>()?,
        is_solid: metadata
            .as_ref()
            .and_then(|d| d.get("is_solid").map(|b| b.as_bool()))
            .flatten(),
        referenced_by_milestone_index: metadata
            .as_ref()
            .and_then(|d| d.get("referenced_by_milestone_index").map(|b| b.as_u32()))
            .transpose()?,
        milestone_index: metadata
            .as_ref()
            .and_then(|d| d.get("milestone_index").map(|b| b.as_u32()))
            .transpose()?,
        should_promote: metadata
            .as_ref()
            .and_then(|d| d.get("should_promote").map(|b| b.as_bool()))
            .flatten(),
        should_reattach: metadata
            .as_ref()
            .and_then(|d| d.get("should_reattach").map(|b| b.as_bool()))
            .flatten(),
        ledger_inclusion_state: metadata
            .as_ref()
            .and_then(|d| d.get("inclusion_state").map(|b| b.as_u8()))
            .transpose()?
            .map(TryInto::try_into)
            .transpose()?,
        conflict_reason: metadata
            .as_ref()
            .and_then(|d| d.get("conflict_reason").map(|b| b.as_u8()))
            .transpose()?,
    })
}

async fn message_children(
    database: Extension<MongoDb>,
    Path(message_id): Path<String>,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
) -> ApiResult<MessageChildrenResponse> {
    let messages = database
        .get_message_children(&MessageId::from_str(&message_id)?, page_size, page)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(MessageChildrenResponse {
        message_id,
        max_results: page_size,
        count: messages.len(),
        children_message_ids: messages
            .into_iter()
            .map(|mut rec| {
                let message = rec.take_document("message")?;
                if expanded {
                    let inclusion_state = rec.get("inclusion_state").map(|b| b.as_u8()).transpose()?;
                    let milestone_index = rec.get("milestone_index").map(|b| b.as_u32()).transpose()?;
                    Ok(Record {
                        id: message.get_as_string("message_id")?,
                        inclusion_state: inclusion_state.map(TryInto::try_into).transpose()?,
                        milestone_index,
                    }
                    .into())
                } else {
                    Ok(message.get_as_string("message_id")?.into())
                }
            })
            .collect::<Result<_, ApiError>>()?,
    })
}

async fn output(database: Extension<MongoDb>, Path(output_id): Path<String>) -> ApiResult<OutputResponse> {
    let output_id = OutputId::from_str(&output_id)?;
    let mut output = database
        .get_output(output_id.transaction_id(), output_id.index())
        .await?
        .ok_or(ApiError::NoResults)?;

    let booked_ms_index = output
        .get_bson("metadata.milestone_index")
        .and_then(|ms| Ok(ms.as_u32()?))?;
    let booked_ms = database
        .get_milestone_record_by_index(booked_ms_index)
        .await?
        .ok_or(ApiError::NoResults)?;
    let spending_transaction = database
        .get_spending_transaction(output_id.transaction_id(), output_id.index())
        .await?;

    let spending_ms_index = spending_transaction.as_ref().and_then(|txn| {
        txn.get_bson("metadata.milestone_index")
            .and_then(|ms| Ok(ms.as_u32()?))
            .ok()
    });
    let spending_ms = if let Some(ms_index) = spending_ms_index {
        database.get_milestone_record_by_index(ms_index).await?
    } else {
        None
    };

    Ok(OutputResponse {
        message_id: output.get_as_string("message_id")?,
        transaction_id: output_id.transaction_id().to_string(),
        output_index: output_id.index(),
        is_spent: spending_transaction.is_some(),
        milestone_index_spent: spending_ms_index,
        milestone_ts_spent: spending_ms.as_ref().and_then(|ms| {
            ms.get_datetime("milestone_timestamp")
                .map(|ms| (ms.timestamp_millis() / 1000) as u32)
                .ok()
        }),
        milestone_index_booked: booked_ms_index,
        milestone_ts_booked: booked_ms
            .get_datetime("milestone_timestamp")
            .map(|ms| (ms.timestamp_millis() / 1000) as u32)?,
        output: output.take_bson("message.payload.data.essence.data.outputs")?.into(),
    })
}

async fn output_metadata(
    database: Extension<MongoDb>,
    Path(output_id): Path<String>,
) -> ApiResult<OutputMetadataResponse> {
    let output_id = OutputId::from_str(&output_id)?;
    let output = database
        .get_output(output_id.transaction_id(), output_id.index())
        .await?
        .ok_or(ApiError::NoResults)?;

    let booked_ms_index = output
        .get_bson("metadata.milestone_index")
        .and_then(|ms| Ok(ms.as_u32()?))?;
    let booked_ms = database
        .get_milestone_record_by_index(booked_ms_index)
        .await?
        .ok_or(ApiError::NoResults)?;
    let spending_transaction = database
        .get_spending_transaction(output_id.transaction_id(), output_id.index())
        .await?;

    let spending_ms_index = spending_transaction.as_ref().and_then(|txn| {
        txn.get_bson("metadata.milestone_index")
            .and_then(|ms| Ok(ms.as_u32()?))
            .ok()
    });
    let spending_ms = if let Some(ms_index) = spending_ms_index {
        database.get_milestone_record_by_index(ms_index).await?
    } else {
        None
    };

    Ok(OutputMetadataResponse {
        message_id: output.get_as_string("message_id")?,
        transaction_id: output_id.transaction_id().to_string(),
        output_index: output_id.index(),
        is_spent: spending_transaction.is_some(),
        milestone_index_spent: spending_ms_index,
        milestone_ts_spent: spending_ms.as_ref().and_then(|ms| {
            ms.get_datetime("milestone_timestamp")
                .map(|ms| (ms.timestamp_millis() / 1000) as u32)
                .ok()
        }),
        transaction_id_spent: spending_transaction.as_ref().and_then(|txn| {
            txn.get_bson("message.payload.transaction_id")
                .and_then(|ms| Ok(ms.as_string()?))
                .ok()
        }),
        milestone_index_booked: booked_ms_index,
        milestone_ts_booked: booked_ms
            .get_datetime("milestone_timestamp")
            .map(|ms| (ms.timestamp_millis() / 1000) as u32)?,
    })
}

async fn transaction_included_message(
    database: Extension<MongoDb>,
    Path(transaction_id): Path<String>,
) -> ApiResult<MessageResponse> {
    let mut rec = database
        .get_message_for_transaction(&transaction_id)
        .await?
        .ok_or(ApiError::NoResults)?;
    let mut message = rec.take_document("message")?;

    Ok(MessageResponse {
        protocol_version: message.get_as_u8("protocol_version")?,
        parents: message
            .take_array("parents")?
            .iter()
            .map(|m| m.as_string())
            .collect::<Result<_, _>>()?,
        payload: message.take_bson("payload").ok().map(Into::into),
        nonce: from_document::<U64>(message.take_document("nonce")?)?.into(),
    })
}

async fn milestone(database: Extension<MongoDb>, Path(milestone_id): Path<String>) -> ApiResult<MilestoneResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id)?;
    database
        .get_milestone_record(&milestone_id)
        .await?
        .ok_or(ApiError::NoResults)
        .and_then(|mut rec| {
            Ok(MilestoneResponse {
                payload: rec.take_bson("message.payload")?.into(),
            })
        })
}

async fn milestone_by_index(database: Extension<MongoDb>, Path(index): Path<u32>) -> ApiResult<MilestoneResponse> {
    database
        .get_milestone_record_by_index(index)
        .await?
        .ok_or(ApiError::NoResults)
        .and_then(|mut rec| {
            Ok(MilestoneResponse {
                payload: rec.take_bson("message.payload")?.into(),
            })
        })
}
