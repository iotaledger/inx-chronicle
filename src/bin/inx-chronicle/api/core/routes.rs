// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Extension, Path},
    handler::Handler,
    http::header::HeaderMap,
    routing::get,
};
use chronicle::{
    db::{
        mongodb::collections::{
            ApplicationStateCollection, BlockCollection, CommittedSlotCollection, OutputCollection, OutputMetadata,
            OutputWithMetadataResult, UtxoChangesResult,
        },
        MongoDb,
    },
    model::block_metadata::BlockMetadata,
};
use iota_sdk::types::{
    api::core::{
        BaseTokenResponse, BlockMetadataResponse, OutputWithMetadataResponse, ProtocolParametersResponse,
        UtxoChangesResponse,
    },
    block::{
        output::{OutputId, OutputMetadata as OutputMetadataResponse},
        payload::signed_transaction::TransactionId,
        slot::{SlotCommitment, SlotCommitmentId, SlotIndex},
        BlockId, SignedBlockDto,
    },
};
use packable::PackableExt;

use super::responses::{InfoResponse, IotaRawResponse, IotaResponse};
use crate::api::{
    error::{ApiError, CorruptStateError, MissingError, RequestError},
    router::Router,
    routes::{is_healthy, not_implemented, BYTE_CONTENT_HEADER},
    ApiResult,
};

pub fn routes() -> Router {
    Router::new()
        .route("/info", get(info))
        .route("/accounts/:account_id/congestion", not_implemented.into_service())
        .route("/rewards/:output_id", not_implemented.into_service())
        .nest(
            "/validators",
            Router::new()
                .route("/", not_implemented.into_service())
                .route("/:account_id", not_implemented.into_service()),
        )
        .route("/committee", not_implemented.into_service())
        .nest(
            "/blocks",
            Router::new()
                .route("/", not_implemented.into_service())
                .route("/:block_id", get(block))
                .route("/:block_id/metadata", get(block_metadata))
                .route("/issuance", not_implemented.into_service()),
        )
        .nest(
            "/outputs",
            Router::new()
                .route("/:output_id", get(output))
                .route("/:output_id/metadata", get(output_metadata))
                .route("/:output_id/full", not_implemented.into_service()),
        )
        .nest(
            "/transactions",
            Router::new()
                .route("/:transaction_id/included-block", get(included_block))
                .route("/:transaction_id/included-block/metadata", get(included_block_metadata)),
        )
        .nest(
            "/commitments",
            Router::new()
                .route("/:commitment_id", get(commitment))
                .route("/:commitment_id/utxo-changes", get(utxo_changes))
                .route("/by-index/:index", get(commitment_by_index))
                .route("/by-index/:index/utxo-changes", get(utxo_changes_by_index)),
        )
        .route("/control/database/prune", not_implemented.into_service())
        .route("/control/snapshot/create", not_implemented.into_service())
}

pub async fn info(database: Extension<MongoDb>) -> ApiResult<InfoResponse> {
    let node_config = database
        .collection::<ApplicationStateCollection>()
        .get_node_config()
        .await?
        .ok_or(CorruptStateError::NodeConfig)?;
    let protocol_parameters = node_config
        .protocol_parameters
        .into_iter()
        .map(|doc| ProtocolParametersResponse {
            parameters: doc.parameters,
            start_epoch: doc.start_epoch,
        })
        .collect::<Vec<_>>();

    let is_healthy = is_healthy(&database).await.unwrap_or_else(|ApiError { error, .. }| {
        tracing::error!("An error occured during health check: {error}");
        false
    });

    let base_token = node_config.base_token;

    let latest_commitment_id = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(CorruptStateError::NodeConfig)?
        .commitment_id;

    Ok(InfoResponse {
        name: chronicle::CHRONICLE_APP_NAME.into(),
        version: std::env!("CARGO_PKG_VERSION").to_string(),
        is_healthy,
        latest_commitment_id,
        base_token: BaseTokenResponse {
            name: base_token.name,
            ticker_symbol: base_token.ticker_symbol,
            decimals: base_token.decimals,
            unit: base_token.unit,
            subunit: base_token.subunit,
            use_metric_prefix: base_token.use_metric_prefix,
        },
        protocol_parameters,
    })
}

async fn block(
    database: Extension<MongoDb>,
    Path(block_id): Path<BlockId>,
    headers: HeaderMap,
) -> ApiResult<IotaRawResponse<SignedBlockDto>> {
    if matches!(headers.get(axum::http::header::ACCEPT), Some(header) if header == BYTE_CONTENT_HEADER) {
        return Ok(IotaRawResponse::Raw(
            database
                .collection::<BlockCollection>()
                .get_block_raw(&block_id)
                .await?
                .ok_or(MissingError::NoResults)?
                .data(),
        ));
    }

    let block = database
        .collection::<BlockCollection>()
        .get_block(&block_id)
        .await?
        .ok_or(MissingError::NoResults)?;

    Ok(IotaRawResponse::Json((&block).into()))
}

fn create_block_metadata_response(block_id: BlockId, metadata: BlockMetadata) -> BlockMetadataResponse {
    BlockMetadataResponse {
        block_id,
        block_state: metadata.block_state,
        transaction_state: metadata.transaction_state,
        block_failure_reason: metadata.block_failure_reason,
        transaction_failure_reason: metadata.transaction_failure_reason,
    }
}

async fn block_metadata(
    database: Extension<MongoDb>,
    Path(block_id_str): Path<String>,
) -> ApiResult<IotaResponse<BlockMetadataResponse>> {
    let block_id = BlockId::from_str(&block_id_str).map_err(RequestError::from)?;
    let metadata = database
        .collection::<BlockCollection>()
        .get_block_metadata(&block_id)
        .await?
        .ok_or(MissingError::NoResults)?;

    Ok(create_block_metadata_response(block_id, metadata).into())
}

fn create_output_metadata_response(
    output_id: OutputId,
    metadata: OutputMetadata,
    latest_commitment_id: SlotCommitmentId,
) -> ApiResult<OutputMetadataResponse> {
    Ok(OutputMetadataResponse::new(
        metadata.block_id,
        output_id,
        metadata.spent_metadata.is_some(),
        metadata.spent_metadata.as_ref().map(|m| m.commitment_id_spent),
        metadata.spent_metadata.as_ref().map(|m| m.transaction_id_spent),
        Some(metadata.included_commitment_id),
        latest_commitment_id,
    ))
}

async fn output(
    database: Extension<MongoDb>,
    Path(output_id): Path<OutputId>,
    headers: HeaderMap,
) -> ApiResult<IotaRawResponse<OutputWithMetadataResponse>> {
    let latest_slot = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(MissingError::NoResults)?;

    let OutputWithMetadataResult {
        output_id,
        output,
        metadata,
    } = database
        .collection::<OutputCollection>()
        .get_output_with_metadata(&output_id, latest_slot.slot_index)
        .await?
        .ok_or(MissingError::NoResults)?;

    if matches!(headers.get(axum::http::header::ACCEPT), Some(header) if header == BYTE_CONTENT_HEADER) {
        return Ok(IotaRawResponse::Raw(output.pack_to_vec()));
    }

    let metadata = create_output_metadata_response(output_id, metadata, latest_slot.commitment_id)?;

    Ok(IotaRawResponse::Json(OutputWithMetadataResponse {
        metadata,
        output: (&output).into(),
    }))
}

async fn output_metadata(
    database: Extension<MongoDb>,
    Path(output_id): Path<String>,
) -> ApiResult<IotaResponse<OutputMetadataResponse>> {
    let latest_slot = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(MissingError::NoResults)?;
    let output_id = OutputId::from_str(&output_id).map_err(RequestError::from)?;
    let metadata = database
        .collection::<OutputCollection>()
        .get_output_metadata(&output_id, latest_slot.slot_index)
        .await?
        .ok_or(MissingError::NoResults)?;

    Ok(create_output_metadata_response(metadata.output_id, metadata.metadata, latest_slot.commitment_id)?.into())
}

async fn included_block(
    database: Extension<MongoDb>,
    Path(transaction_id): Path<TransactionId>,
    headers: HeaderMap,
) -> ApiResult<IotaRawResponse<SignedBlockDto>> {
    if matches!(headers.get(axum::http::header::ACCEPT), Some(header) if header == BYTE_CONTENT_HEADER) {
        return Ok(IotaRawResponse::Raw(
            database
                .collection::<BlockCollection>()
                .get_block_raw_for_transaction(&transaction_id)
                .await?
                .ok_or(MissingError::NoResults)?
                .data(),
        ));
    }

    let block = database
        .collection::<BlockCollection>()
        .get_block_for_transaction(&transaction_id)
        .await?
        .ok_or(MissingError::NoResults)?
        .block;

    Ok(IotaRawResponse::Json((&block).into()))
}

async fn included_block_metadata(
    database: Extension<MongoDb>,
    Path(transaction_id): Path<String>,
) -> ApiResult<IotaResponse<BlockMetadataResponse>> {
    let transaction_id = TransactionId::from_str(&transaction_id).map_err(RequestError::from)?;

    let res = database
        .collection::<BlockCollection>()
        .get_block_metadata_for_transaction(&transaction_id)
        .await?
        .ok_or(MissingError::NoResults)?;
    let block_id = res.block_id;
    let metadata = res.metadata;

    Ok(create_block_metadata_response(block_id, metadata).into())
}

async fn commitment(
    database: Extension<MongoDb>,
    Path(commitment_id): Path<SlotCommitmentId>,
    headers: HeaderMap,
) -> ApiResult<IotaRawResponse<SlotCommitment>> {
    commitment_by_index(database, Path(commitment_id.slot_index()), headers).await
}

async fn commitment_by_index(
    database: Extension<MongoDb>,
    Path(index): Path<SlotIndex>,
    headers: HeaderMap,
) -> ApiResult<IotaRawResponse<SlotCommitment>> {
    let slot_commitment = database
        .collection::<CommittedSlotCollection>()
        .get_commitment(index)
        .await?
        .ok_or(MissingError::NoResults)?;

    if matches!(headers.get(axum::http::header::ACCEPT), Some(header) if header == BYTE_CONTENT_HEADER) {
        return Ok(IotaRawResponse::Raw(slot_commitment.commitment.data()));
    }

    Ok(IotaRawResponse::Json(slot_commitment.commitment.into_inner()))
}

async fn utxo_changes(
    database: Extension<MongoDb>,
    Path(commitment_id): Path<SlotCommitmentId>,
) -> ApiResult<IotaResponse<UtxoChangesResponse>> {
    utxo_changes_by_index(database, Path(commitment_id.slot_index())).await
}

async fn utxo_changes_by_index(
    database: Extension<MongoDb>,
    Path(index): Path<SlotIndex>,
) -> ApiResult<IotaResponse<UtxoChangesResponse>> {
    let latest_slot = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(MissingError::NoResults)?;

    let UtxoChangesResult {
        created_outputs,
        consumed_outputs,
    } = database
        .collection::<OutputCollection>()
        .get_utxo_changes(index, latest_slot.slot_index)
        .await?
        .ok_or(MissingError::NoResults)?;

    Ok(UtxoChangesResponse {
        index: index.0,
        created_outputs,
        consumed_outputs,
    }
    .into())
}
