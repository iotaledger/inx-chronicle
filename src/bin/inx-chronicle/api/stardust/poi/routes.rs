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
        collections::{BlockCollection, MilestoneCollection},
        MongoDb,
    },
    types::stardust::block::BlockId,
};
use crypto::hashes::blake2b::Blake2b256;

use super::{
    error::PoIError,
    hasher::MerkleHasher,
    proof::Proof,
    responses::{CreateProofResponse, ValidateProofResponse},
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
    let merkle_hasher = MerkleHasher::<Blake2b256>::new();
    let proof = merkle_hasher.create_proof(&block_ids, &block_id)?;

    // Fetch the corresponding milestone to return in the response.
    let milestone_collection = database.collection::<MilestoneCollection>();
    let milestone = milestone_collection
        .get_milestone_payload(referenced_index)
        .await?
        .ok_or(ApiError::NoResults)?;

    let inclusion_merkle_root = milestone.essence.inclusion_merkle_root;
    if &*proof.hash(&merkle_hasher) != &inclusion_merkle_root {
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
    let block = iota_types::block::Block::try_from_dto_unverified(&block)
        .map_err(|_| ApiError::PoI(PoIError::InvalidRequest("malformed block")))?;
    let block_id = block.id().into();

    let hasher = MerkleHasher::<Blake2b256>::new();

    let proof = Proof::try_from(proof).map_err(|_| ApiError::PoI(PoIError::InvalidRequest("malformed proof")))?;
    if !proof.contains_block_id(&block_id, &hasher)
    // .map_err(|_| PoIError::InvalidProof(block_id.to_hex()))?
    {
        Ok(ValidateProofResponse { valid: false })
    } else {
        let inclusion_merkle_root = milestone.inclusion_merkle_root;

        // todo!("verify the contained milestone signatures");
        let signatures = milestone.signatures;

        Ok(ValidateProofResponse {
            valid: hasher.validate_proof(proof)?,
        })
    }
}
