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
        stardust::block::{BlockId, MilestoneId, OutputId, Payload, TransactionId, TransactionEssence},
        tangle::MilestoneIndex, ledger::LedgerInclusionState,
    },
};
use futures::TryStreamExt;

use super::responses::{bee, *};
use crate::api::{
    error::{ApiError, ParseError},
    extractors::{Expanded, Pagination},
    responses::{Expansion, Record},
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
                .route("/by-index/:index", get(milestone_by_index))
                .route("/:milestone_id/utxo-changes", get(utxo_changes))
                .route("/by-index/:index/utxo-changes", get(utxo_changes_by_index)),
        )
}

async fn block(database: Extension<MongoDb>, Path(block_id): Path<String>) -> ApiResult<BlockResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let block = database.get_block(&block_id).await?.ok_or(ApiError::NoResults)?;

    Ok(BlockResponse(bee::BlockResponse(bee::BlockDto {
        protocol_version: block.protocol_version,
        parents: block.parents.iter().map(|b| b.to_hex()).collect(),
        payload: block.payload.map(|p| {
            // TODO: unwrap
            let bee_payload: &bee::Payload = &p.try_into().unwrap();
            bee_payload.into()
        }),
        nonce: block.nonce.to_string(),
    })))
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

    Ok(BlockMetadataResponse(bee::BlockMetadataResponse {
        block_id: metadata.block_id.to_hex(),
        parents: metadata.parents.iter().map(|id| id.to_hex()).collect(),
        is_solid: metadata.is_solid,
        referenced_by_milestone_index: Some(*metadata.referenced_by_milestone_index),
        milestone_index: Some(*metadata.milestone_index),
        ledger_inclusion_state: Some(convert_ledger_inclusion_state(metadata.inclusion_state)),
        conflict_reason: Some(metadata.conflict_reason as u8),
        should_promote: Some(metadata.should_promote),
        should_reattach: Some(metadata.should_reattach),
    }))
}

fn convert_ledger_inclusion_state(s: LedgerInclusionState) -> bee::LedgerInclusionStateDto {
    match s {
        LedgerInclusionState::Conflicting => bee::LedgerInclusionStateDto::Conflicting,
        LedgerInclusionState::Included => bee::LedgerInclusionStateDto::Included,
        LedgerInclusionState::NoTransaction => bee::LedgerInclusionStateDto::NoTransaction,
    }
}

async fn block_children(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<BlockChildrenResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let children = database
        .get_block_children(&block_id, page_size, page)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(BlockChildrenResponse(bee::BlockChildrenResponse {
        block_id: block_id.to_hex(),
        max_results: page_size,
        count: children.len(),
        children: children
            .into_iter()
            .map(|block_id| block_id.to_hex())
            .collect(),
    }))
}

// {
//     "metadata": {
//       "blockId": "0x9cd745ef6800c8e8c80b09174ee4b250b3c43dfa62d7c6a4e61f848febf731a0",
//       "transactionId": "0x1ee46e19f4219ee65afc10227d0ca22753f76ef32d1e922e5cbe3fbc9b5a5298",
//       "outputIndex": 1,
//       "isSpent": false,
//       "milestoneIndexBooked": 1234567,
//       "milestoneTimestampBooked": 1643207146,
//       "ledgerIndex": 946704
//     },
//     "output": {
//       "type": 3,
//       "amount": "1000",
//       "unlockConditions": [
//         {
//           "type": 0,
//           "address": {
//             "type": 0,
//             "pubKeyHash": "0x8eaf87ac1f52eb05f2c7c0c15502df990a228838dc37bd18de9503d69afd257d"
//           }
//         }
//       ]
//     }
// }
async fn output(database: Extension<MongoDb>, Path(output_id): Path<String>) -> ApiResult<OutputResponse> {
    let output_id = OutputId::from_str(&output_id).map_err(ApiError::bad_parse)?;
    let (output, metadata) = database
        .get_output_and_metadata(&output_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    let OutputId { index: output_index, ..} = output_id;

    let metadata = bee::OutputMetadataResponse {
        block_id: metadata.block_id.to_hex(),
        transaction_id: metadata.transaction_id.to_hex(),
        output_index,
        is_spent: metadata.spent.is_some(),
        milestone_index_spent: metadata.spent.as_ref().map(|spent_md| *spent_md.spent.milestone_index),
        milestone_timestamp_spent: metadata.spent.as_ref().map(|spent_md| *spent_md.spent.milestone_timestamp),
        transaction_id_spent: metadata.spent.as_ref().map(|spent_md| spent_md.transaction_id.to_hex()),
        milestone_index_booked: *metadata.booked.milestone_index,
        milestone_timestamp_booked: *metadata.booked.milestone_timestamp,
        // TODO: return proper value
        ledger_index: 0,
    };

    // TODO: introduce ApiError::Conversion?
    let output: &bee::Output = &output.try_into().map_err(|_| ApiError::NoResults)?;
    let output: bee::OutputDto = output.into();

    Ok(OutputResponse(bee::OutputResponse {
        metadata,
        output,
    }))
}

// Example:
// {
//     "blockId": "0x9cd745ef6800c8e8c80b09174ee4b250b3c43dfa62d7c6a4e61f848febf731a0",
//     "transactionId": "0x1ee46e19f4219ee65afc10227d0ca22753f76ef32d1e922e5cbe3fbc9b5a5298",
//     "outputIndex": 1,
//     "isSpent": false,
//     "milestoneIndexBooked": 1234567,
//     "milestoneTimestampBooked": 1643207146,
//     "ledgerIndex": 946704
// }
async fn output_metadata(
    database: Extension<MongoDb>,
    Path(output_id): Path<String>,
) -> ApiResult<OutputMetadataResponse> {
    let output_id = OutputId::from_str(&output_id).map_err(ApiError::bad_parse)?;
    let metadata = database
        .get_output_metadata(&output_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    let OutputId { index: output_index, ..} = output_id;

    Ok(OutputMetadataResponse(bee::OutputMetadataResponse {
        block_id: metadata.block_id.to_hex(),
        transaction_id: metadata.transaction_id.to_hex(),
        output_index,
        is_spent: metadata.spent.is_some(),
        milestone_index_spent: metadata.spent.as_ref().map(|spent_md| *spent_md.spent.milestone_index),
        milestone_timestamp_spent: metadata.spent.as_ref().map(|spent_md| *spent_md.spent.milestone_timestamp),
        transaction_id_spent: metadata.spent.as_ref().map(|spent_md| spent_md.transaction_id.to_hex()),
        milestone_index_booked: *metadata.booked.milestone_index,
        milestone_timestamp_booked: *metadata.booked.milestone_timestamp,
        // TODO: return proper value
        ledger_index: 0,
    }))
}

async fn transaction_included_block(
    database: Extension<MongoDb>,
    Path(transaction_id): Path<String>,
) -> ApiResult<BlockResponse> {
    let transaction_id = TransactionId::from_str(&transaction_id).map_err(ApiError::bad_parse)?;
    let block = database
        .get_block_for_transaction(&transaction_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(BlockResponse(bee::BlockResponse(bee::BlockDto {
        protocol_version: block.protocol_version,
        parents: block.parents.iter().map(|b| b.to_hex()).collect(),
        payload: block.payload.map(|p| {
            // Unwrap: TODO
            let bee_payload: &bee::Payload = &p.try_into().unwrap();
            bee_payload.into()
        }),
        nonce: block.nonce.to_string(),
    })))
}

async fn receipts(
    database: Extension<MongoDb>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<ReceiptsResponse> {
    todo!("receipts")
}

async fn receipts_migrated_at(
    database: Extension<MongoDb>,
    Path(index): Path<u32>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<ReceiptsResponse> {
    todo!("receipts")
}

async fn treasury(
    database: Extension<MongoDb>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<TreasuryResponse> {
    todo!("treasury")
}

async fn milestone(database: Extension<MongoDb>, Path(milestone_id): Path<String>) -> ApiResult<MilestoneResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(ApiError::bad_parse)?;
    database
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(ApiError::NoResults)
        .map(|payload| {
            // TODO: unwrap
            let payload: &bee::MilestonePayload = &payload.try_into().unwrap();
            let payload_dto = payload.into();
            MilestoneResponse(bee::MilestoneResponse(payload_dto))
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
            let payload: &bee::MilestonePayload = &payload.try_into().unwrap();
            let payload_dto = payload.into();
            MilestoneResponse(bee::MilestoneResponse(payload_dto))
        })
}

async fn utxo_changes(database: Extension<MongoDb>, Path(milestone_id): Path<String>) -> ApiResult<UtxoChangesResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(ApiError::bad_parse)?;
    let payload = database
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    todo!("utxo_changes")
}

async fn utxo_changes_by_index(database: Extension<MongoDb>, Path(index): Path<MilestoneIndex>) -> ApiResult<UtxoChangesResponse> {
    let payload = database
        .get_milestone_payload(index)
        .await?
        .ok_or(ApiError::NoResults)?;

    todo!("utxo_changes_by_index")
}
