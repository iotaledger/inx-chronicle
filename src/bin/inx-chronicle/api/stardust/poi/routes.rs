// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Json, Path},
    routing::{get, post},
    Extension,
};
use chronicle::{
    db::{
        collections::{BlockCollection, ConfigurationUpdateCollection, MilestoneCollection},
        MongoDb,
    },
    types::stardust::block::BlockId,
};
use crypto::hashes::blake2b::Blake2b256;

use super::{
    error::PoIError,
    merkle_hasher::MerkleHasher,
    merkle_proof::MerkleProof,
    responses::{CreateProofResponse, ValidateProofResponse},
    verification::MilestoneKeyManager,
};
use crate::api::{error::InternalApiError, router::Router, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .route("/create/:block_id", get(create_proof))
        .route("/validate", post(validate_proof))
}

async fn create_proof(database: Extension<MongoDb>, Path(block_id): Path<String>) -> ApiResult<CreateProofResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let block_collection = database.collection::<BlockCollection>();

    // Ensure the corresponding block was referenced by a milestone.
    let block_metadata = block_collection
        .get_block_metadata(&block_id)
        .await?
        .ok_or(ApiError::NoResults)?;
    let referenced_index = block_metadata.referenced_by_milestone_index;
    if referenced_index == 0 {
        return Err(ApiError::PoI(PoIError::InvalidRequest("block not referenced")));
    }

    // Fetch the corresponding milestone cone in "White Flag" order.
    let block_ids = block_collection
        .get_pastcone_in_white_flag_order(referenced_index)
        .await?;
    if block_ids.is_empty() {
        return Err(ApiError::Internal(InternalApiError::CorruptState(
            "missing past-cone of referencing milestone",
        )));
    }

    // Create the inclusion proof to return in the response.
    let hasher = MerkleHasher::<Blake2b256>::new();
    let proof = hasher.create_proof(&block_ids, &block_id)?;

    // Fetch the corresponding milestone to return in the response.
    let milestone_collection = database.collection::<MilestoneCollection>();
    let milestone = milestone_collection
        .get_milestone_payload(referenced_index)
        .await?
        .ok_or(ApiError::NoResults)?;

    let inclusion_merkle_root = milestone.essence.inclusion_merkle_root;
    if *proof.hash(&hasher) != inclusion_merkle_root {
        return Err(ApiError::PoI(PoIError::InvalidProof(
            "cannot create a valid proof for that block".to_string(),
        )));
    }

    // Fetch the corresponding block to return in the response.
    let block = block_collection
        .get_block(&block_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(CreateProofResponse {
        milestone: milestone.into(),
        block: block.into(),
        proof: proof.into(),
    })
}

async fn validate_proof(
    database: Extension<MongoDb>,
    Json(CreateProofResponse {
        milestone,
        block,
        proof,
    }): Json<CreateProofResponse>,
) -> ApiResult<ValidateProofResponse> {
    // Extract the block, milestone, and proof.
    let block = iota_types::block::Block::try_from_dto_unverified(&block)
        .map_err(|_| ApiError::PoI(PoIError::InvalidRequest("malformed block")))?;
    let milestone = iota_types::block::payload::milestone::MilestonePayload::try_from_dto_unverified(&milestone)
        .map_err(|_| ApiError::PoI(PoIError::InvalidRequest("malformed milestone")))?;
    let proof = MerkleProof::try_from(proof).map_err(|_| ApiError::PoI(PoIError::InvalidRequest("malformed proof")))?;

    // Fetch the corresponding block referenced index.
    let block_collection = database.collection::<BlockCollection>();
    let block_id = block.id().into();
    let block_referenced_index = block_collection
        .get_block_metadata(&block_id)
        .await?
        .ok_or(ApiError::NoResults)?
        .referenced_by_milestone_index;

    // Fetch the corresponding milestone to return in the response.
    let milestone_id = milestone.id().into();
    let milestone_collection = database.collection::<MilestoneCollection>();
    let milestone_index = milestone_collection
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(ApiError::NoResults)?
        .essence
        .index;

    if block_referenced_index != milestone_index {
        return Err(ApiError::PoI(PoIError::InvalidProof(
            "block not referenced by given milestone".to_string(),
        )));
    }

    let hasher = MerkleHasher::<Blake2b256>::new();

    // Fetch the node configuration.
    let update_collection = database.collection::<ConfigurationUpdateCollection>();
    let node_configuration = update_collection
        .get_node_configuration_for_ledger_index(milestone_index)
        .await?
        .ok_or(ApiError::NoResults)?
        .config;

    let public_key_count = node_configuration.milestone_public_key_count as usize;
    let key_ranges = node_configuration.milestone_key_ranges;
    let key_manager = MilestoneKeyManager::new(key_ranges);
    let applicable_public_keys = key_manager.get_valid_public_keys_for_index(milestone_index);

    let valid = proof.contains_block_id(&block_id, &hasher)
        && milestone
            .validate(&applicable_public_keys, public_key_count)
            .map_err(|_| PoIError::InvalidProof("milestone validation error".to_string()))?
            .eq(&())
        && *proof.hash(&hasher) == **milestone.essence().inclusion_merkle_root();

    Ok(ValidateProofResponse { valid })
}
