// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Path, State},
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
        TransactionMetadataResponse, UtxoChangesResponse,
    },
    block::{
        output::{
            OutputConsumptionMetadata, OutputId, OutputInclusionMetadata, OutputMetadata as OutputMetadataResponse,
        },
        payload::signed_transaction::TransactionId,
        slot::{SlotCommitment, SlotCommitmentId, SlotIndex},
        BlockDto, BlockId,
    },
};
use packable::PackableExt;

use super::responses::{InfoResponse, IotaRawResponse, IotaResponse};
use crate::api::{
    error::{ApiError, CorruptStateError, MissingError, RequestError},
    router::Router,
    routes::{is_healthy, not_implemented, BYTE_CONTENT_HEADER},
    ApiResult, ApiState,
};

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/info", get(info))
        .route("/accounts/:account_id/congestion", get(not_implemented))
        .route("/rewards/:output_id", get(not_implemented))
        .nest(
            "/validators",
            Router::new()
                .route("/", get(not_implemented))
                .route("/:account_id", get(not_implemented)),
        )
        .route("/committee", get(not_implemented))
        .nest(
            "/blocks",
            Router::new()
                .route("/", get(not_implemented))
                .route("/:block_id", get(block))
                .route("/:block_id/metadata", get(block_metadata))
                .route("/issuance", get(not_implemented)),
        )
        .nest(
            "/outputs",
            Router::new()
                .route("/:output_id", get(output))
                .route("/:output_id/metadata", get(output_metadata))
                .route("/:output_id/full", get(not_implemented)),
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
}

pub async fn info(database: State<MongoDb>) -> ApiResult<InfoResponse> {
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
        },
        protocol_parameters,
    })
}

async fn block(
    database: State<MongoDb>,
    Path(block_id): Path<BlockId>,
    headers: HeaderMap,
) -> ApiResult<IotaRawResponse<BlockDto>> {
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
        block_state: metadata.block_state.into(),
        block_failure_reason: metadata.block_failure_reason.map(Into::into),
        transaction_metadata: metadata
            .transaction_metadata
            .map(|metadata| TransactionMetadataResponse {
                transaction_id: metadata.transaction_id,
                transaction_state: metadata.transaction_state.into(),
                transaction_failure_reason: metadata.transaction_failure_reason.map(Into::into),
            }),
    }
}

async fn block_metadata(
    database: State<MongoDb>,
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
        output_id,
        metadata.block_id,
        OutputInclusionMetadata::new(
            metadata.commitment_id_included.slot_index(),
            *output_id.transaction_id(),
            Some(metadata.commitment_id_included),
        ),
        metadata.spent_metadata.map(|metadata| {
            OutputConsumptionMetadata::new(
                metadata.slot_spent,
                metadata.transaction_id_spent,
                Some(metadata.commitment_id_spent),
            )
        }),
        latest_commitment_id,
    ))
}

async fn output(
    database: State<MongoDb>,
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

    Ok(IotaRawResponse::Json(OutputWithMetadataResponse { metadata, output }))
}

async fn output_metadata(
    database: State<MongoDb>,
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
    database: State<MongoDb>,
    Path(transaction_id): Path<TransactionId>,
    headers: HeaderMap,
) -> ApiResult<IotaRawResponse<BlockDto>> {
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
    database: State<MongoDb>,
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
    database: State<MongoDb>,
    Path(commitment_id): Path<SlotCommitmentId>,
    headers: HeaderMap,
) -> ApiResult<IotaRawResponse<SlotCommitment>> {
    commitment_by_index(database, Path(commitment_id.slot_index()), headers).await
}

async fn commitment_by_index(
    database: State<MongoDb>,
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
    database: State<MongoDb>,
    Path(commitment_id): Path<SlotCommitmentId>,
) -> ApiResult<IotaResponse<UtxoChangesResponse>> {
    utxo_changes_by_index(database, Path(commitment_id.slot_index())).await
}

async fn utxo_changes_by_index(
    database: State<MongoDb>,
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
        commitment_id: latest_slot.commitment_id,
        created_outputs,
        consumed_outputs,
    }
    .into())
}
