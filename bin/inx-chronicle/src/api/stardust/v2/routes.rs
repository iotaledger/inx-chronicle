// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Extension, Path},
    handler::Handler,
    http::header::{HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::*,
    Router,
};
use bee_block_stardust::{output::dto::OutputDto, payload::milestone::option::dto::MilestoneOptionDto, BlockDto};
use bee_rest_api_stardust::types::{
    dtos::{LedgerInclusionStateDto, ReceiptDto},
    responses::{
        BlockChildrenResponse, BlockMetadataResponse, BlockResponse, MilestoneResponse, OutputMetadataResponse,
        OutputResponse, ReceiptsResponse, TreasuryResponse, UtxoChangesResponse,
    },
};
use chronicle::{
    db::MongoDb,
    types::{
        ledger::{LedgerInclusionState, OutputWithMetadata},
        stardust::block::{BlockId, MilestoneId, MilestoneOption, OutputId, TransactionId},
        tangle::MilestoneIndex,
    },
};
use derive_more::From;
use futures::TryStreamExt;
use lazy_static::lazy_static;
use mongodb::bson;
use serde::Serialize;

use crate::api::{error::ApiError, extractors::Pagination, routes::not_implemented, ApiResult};

lazy_static! {
    pub(crate) static ref BYTE_CONTENT_HEADER: HeaderValue =
        HeaderValue::from_str("application/vnd.iota.serializer-v1").unwrap();
}

pub fn routes() -> Router {
    Router::new()
        .route("/info", not_implemented.into_service())
        .route("/tips", not_implemented.into_service())
        .route("/blocks", not_implemented.into_service())
        .nest(
            "/blocks",
            Router::new()
                .route("/:block_id", get(block))
                .route("/:block_id/children", get(block_children))
                .route("/:block_id/metadata", get(block_metadata)),
        )
        .nest(
            "/outputs",
            Router::new()
                .route("/:output_id", get(output))
                .route("/:output_id/metadata", get(output_metadata)),
        )
        .route("/receipts", get(receipts))
        .nest(
            "/receipts",
            Router::new().route("/:migrated_at", get(receipts_migrated_at)),
        )
        .route("/treasury", get(treasury))
        .nest(
            "/transactions",
            Router::new().route("/:transaction_id/included-block", get(transaction_included_block)),
        )
        .nest(
            "/milestones",
            Router::new()
                .route("/:milestone_id", get(milestone))
                .route("/:milestone_id/utxo-changes", get(utxo_changes))
                .route("/by-index/:index", get(milestone_by_index))
                .route("/by-index/:index/utxo-changes", get(utxo_changes_by_index)),
        .route(path "/peers", not_implemented.into_service()
        .nest(
            "/peers",
            Router::new()
                .route("/:peer_id", not_implemented.into_service()),
        )
        .route("/control/database/prune", not_implemented.into_service()),
        .route("/control/snapshot/create", not_implemented.into_service()),
}

#[derive(Debug, Serialize, From)]
enum BlockOrRawResponse {
    Block(BlockResponse),
    Raw(Vec<u8>),
}

async fn block(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<BlockOrRawResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let block = database.get_block(&block_id).await?.ok_or(ApiError::NoResults)?;

    if let Some(value) = headers.get(axum::http::header::ACCEPT) {
        if value.eq(&*BYTE_CONTENT_HEADER) {
            return Ok(database
                .get_block_raw(&block_id)
                .await?
                .ok_or(ApiError::NoResults)?
                .into());
        }
    }

    Ok(BlockResponse(BlockDto {
        protocol_version: block.protocol_version,
        parents: block.parents.iter().map(|b| b.to_hex()).collect(),
        payload: block.payload.map(|p| {
            // TODO: unwrap
            let bee_payload: &bee_block_stardust::payload::Payload = &p.try_into().unwrap();
            bee_payload.into()
        }),
        nonce: block.nonce.to_string(),
    })
    .into())
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
        is_solid: metadata.is_solid,
        referenced_by_milestone_index: Some(*metadata.referenced_by_milestone_index),
        milestone_index: Some(*metadata.milestone_index),
        ledger_inclusion_state: Some(convert_ledger_inclusion_state(metadata.inclusion_state)),
        conflict_reason: Some(metadata.conflict_reason as u8),
        should_promote: Some(metadata.should_promote),
        should_reattach: Some(metadata.should_reattach),
    })
}

fn convert_ledger_inclusion_state(s: LedgerInclusionState) -> LedgerInclusionStateDto {
    match s {
        LedgerInclusionState::Conflicting => LedgerInclusionStateDto::Conflicting,
        LedgerInclusionState::Included => LedgerInclusionStateDto::Included,
        LedgerInclusionState::NoTransaction => LedgerInclusionStateDto::NoTransaction,
    }
}

async fn block_children(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<BlockChildrenResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let mut block_children = database
        .get_block_children(&block_id, page_size, page)
        .await
        .map_err(|_| ApiError::NoResults)?;

    let mut children = Vec::new();
    while let Some(block_id) = block_children.try_next().await? {
        children.push(block_id.to_hex());
    }

    Ok(BlockChildrenResponse {
        block_id: block_id.to_hex(),
        max_results: page_size,
        count: children.len(),
        children,
    })
}

async fn output(database: Extension<MongoDb>, Path(output_id): Path<String>) -> ApiResult<OutputResponse> {
    let output_id = OutputId::from_str(&output_id).map_err(ApiError::bad_parse)?;
    let OutputWithMetadata { output, metadata } = database
        .get_output_with_metadata(&output_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    let OutputId {
        index: output_index, ..
    } = output_id;

    let metadata = OutputMetadataResponse {
        block_id: metadata.block_id.to_hex(),
        transaction_id: metadata.transaction_id.to_hex(),
        output_index,
        is_spent: metadata.spent.is_some(),
        milestone_index_spent: metadata.spent.as_ref().map(|spent_md| *spent_md.spent.milestone_index),
        // TODO: can assume that the information always exists in Chronicle?
        milestone_timestamp_spent: metadata
            .spent
            .as_ref()
            .map(|spent_md| *spent_md.spent.milestone_timestamp),
        transaction_id_spent: metadata.spent.as_ref().map(|spent_md| spent_md.transaction_id.to_hex()),
        milestone_index_booked: *metadata.booked.milestone_index,
        // TODO: can assume that the information always exists in Chronicle?
        milestone_timestamp_booked: *metadata.booked.milestone_timestamp,
        // TODO: return proper value
        ledger_index: 0,
    };

    // TODO: introduce ApiError::Conversion?
    let output: &bee_block_stardust::output::Output = &output.try_into().map_err(|_| ApiError::NoResults)?;
    let output: OutputDto = output.into();

    Ok(OutputResponse { metadata, output })
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

    let OutputId {
        index: output_index, ..
    } = output_id;

    Ok(OutputMetadataResponse {
        block_id: metadata.block_id.to_hex(),
        transaction_id: metadata.transaction_id.to_hex(),
        output_index,
        is_spent: metadata.spent.is_some(),
        milestone_index_spent: metadata.spent.as_ref().map(|spent_md| *spent_md.spent.milestone_index),
        // TODO: can assume that the information always exists in Chronicle?
        milestone_timestamp_spent: metadata
            .spent
            .as_ref()
            .map(|spent_md| *spent_md.spent.milestone_timestamp),
        transaction_id_spent: metadata.spent.as_ref().map(|spent_md| spent_md.transaction_id.to_hex()),
        milestone_index_booked: *metadata.booked.milestone_index,
        milestone_timestamp_booked: *metadata.booked.milestone_timestamp,
        // TODO: return proper value
        ledger_index: 0,
    })
}

async fn transaction_included_block(
    database: Extension<MongoDb>,
    Path(_): Path<u32>,
    Path(transaction_id): Path<String>,
) -> ApiResult<BlockResponse> {
    let transaction_id = TransactionId::from_str(&transaction_id).map_err(ApiError::bad_parse)?;
    let block = database
        .get_block_for_transaction(&transaction_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(BlockResponse(BlockDto {
        protocol_version: block.protocol_version,
        parents: block.parents.iter().map(|b| b.to_hex()).collect(),
        payload: block.payload.map(|p| {
            // Unwrap: TODO
            let bee_payload: &bee_block_stardust::payload::Payload = &p.try_into().unwrap();
            bee_payload.into()
        }),
        nonce: block.nonce.to_string(),
    }))
}

async fn receipts(
    database: Extension<MongoDb>,
    Pagination { page_size: _, page: _ }: Pagination,
) -> ApiResult<ReceiptsResponse> {
    let mut milestone_options = database.get_milestone_options().await?;
    let mut receipts = Vec::new();
    while let Some(doc) = milestone_options.try_next().await? {
        // TODO: unwrap
        let (index, opt): (MilestoneIndex, MilestoneOption) = bson::from_document(doc).unwrap();
        let opt: &bee_block_stardust::payload::milestone::MilestoneOption = &opt.try_into().unwrap();
        let opt: MilestoneOptionDto = opt.into();

        if let MilestoneOptionDto::Receipt(receipt) = opt {
            receipts.push(ReceiptDto {
                receipt,
                milestone_index: *index,
            });
        }
    }
    Ok(ReceiptsResponse { receipts })
}

async fn receipts_migrated_at(database: Extension<MongoDb>, Path(index): Path<u32>) -> ApiResult<ReceiptsResponse> {
    let mut milestone_options = database.get_milestone_options_migrated_at(index.into()).await?;
    let mut receipts = Vec::new();
    while let Some(doc) = milestone_options.try_next().await? {
        // TODO: unwrap
        let (index, opt): (MilestoneIndex, MilestoneOption) = bson::from_document(doc).unwrap();
        let opt: &bee_block_stardust::payload::milestone::MilestoneOption = &opt.try_into().unwrap();
        let opt: MilestoneOptionDto = opt.into();

        if let MilestoneOptionDto::Receipt(receipt) = opt {
            receipts.push(ReceiptDto {
                receipt,
                milestone_index: *index,
            });
        }
    }
    Ok(ReceiptsResponse { receipts })
}

async fn treasury(database: Extension<MongoDb>) -> ApiResult<TreasuryResponse> {
    database
        .get_latest_treasury()
        .await?
        .ok_or(ApiError::NoResults)
        .map(|treasury| TreasuryResponse {
            milestone_id: treasury.milestone_id.to_hex(),
            amount: treasury.amount.to_string(),
        })
}

async fn milestone(database: Extension<MongoDb>, Path(milestone_id): Path<String>) -> ApiResult<MilestoneResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(ApiError::bad_parse)?;
    database
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(ApiError::NoResults)
        .map(|payload| {
            // TODO: unwrap
            let payload: &bee_block_stardust::payload::milestone::MilestonePayload = &payload.try_into().unwrap();
            let payload_dto = payload.into();
            MilestoneResponse(payload_dto)
        })
}

async fn milestone_by_index(
    database: Extension<MongoDb>,
    Path(index): Path<MilestoneIndex>,
) -> ApiResult<MilestoneResponse> {
    database
        .get_milestone_payload(index)
        .await?
        .ok_or(ApiError::NoResults)
        .map(|payload| {
            // TODO: unwrap
            let payload: &bee_block_stardust::payload::milestone::MilestonePayload = &payload.try_into().unwrap();
            let payload_dto = payload.into();
            MilestoneResponse(payload_dto)
        })
}

async fn utxo_changes(
    database: Extension<MongoDb>,
    Path(milestone_id): Path<String>,
) -> ApiResult<UtxoChangesResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(ApiError::bad_parse)?;
    let milestone_index = database
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(ApiError::NoResults)?
        .essence
        .index;
    collect_utxo_changes(&database, milestone_index).await
}

async fn utxo_changes_by_index(
    database: Extension<MongoDb>,
    Path(milestone_index): Path<MilestoneIndex>,
) -> ApiResult<UtxoChangesResponse> {
    collect_utxo_changes(&database, milestone_index).await
}

async fn collect_utxo_changes(database: &MongoDb, milestone_index: MilestoneIndex) -> ApiResult<UtxoChangesResponse> {
    let mut created_outputs = Vec::new();
    let mut consumed_outputs = Vec::new();

    let mut updates = database.get_ledger_updates_at_index(milestone_index).await?;
    while let Some(update) = updates.try_next().await? {
        if update.is_spent {
            consumed_outputs.push(update.output_id.to_hex());
        } else {
            created_outputs.push(update.output_id.to_hex());
        }
    }

    Ok(UtxoChangesResponse {
        index: *milestone_index,
        created_outputs,
        consumed_outputs,
    })
}
