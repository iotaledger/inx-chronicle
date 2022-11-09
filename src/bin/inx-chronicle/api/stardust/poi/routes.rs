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
    hasher::MerkleTreeHasher,
    responses::{CreateProofResponse, ValidateProofResponse}, proof::Proof,
};
use crate::api::{error::InternalApiError, router::Router, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .route("/create/:block_id", get(create_proof))
        .route("/validate", post(validate_proof))
}

async fn create_proof(database: Extension<MongoDb>, Path(block_id): Path<String>) -> ApiResult<CreateProofResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;

    // Fetch the whole milestone cone in "White Flag" order which the given block also belongs to.
    let block_collection = database.collection::<BlockCollection>();
    let block = block_collection
        .get_block(&block_id)
        .await?
        .ok_or(ApiError::NoResults)?;
    let block_metadata = block_collection
        .get_block_metadata(&block_id)
        .await?
        .ok_or(ApiError::NoResults)?;
    let referenced_index = block_metadata.referenced_by_milestone_index;
    if referenced_index == 0 {
        return Err(ApiError::PoI(PoIError::InvalidRequest("block not referenced")));
    }
    let block_ids = block_collection
        .get_pastcone_in_white_flag_order(referenced_index)
        .await?;
    if block_ids.is_empty() {
        return Err(ApiError::Internal(InternalApiError::CorruptState(
            "missing past-cone of referencing milestone",
        )));
    }

    //
    let merkle_hasher = MerkleTreeHasher::<Blake2b256>::new();
    let proof = merkle_hasher.create_proof(block_ids, &block_id)?;

    let milestone_collection = database.collection::<MilestoneCollection>();
    let milestone = milestone_collection
        .get_milestone_payload(referenced_index)
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
    let proof = Proof::try_from(proof).unwrap();

    if !proof
        .contains_block_id(&block_id)
        .map_err(|_| PoIError::InvalidProof(block_id.to_hex()))?
    {
        return Ok(ValidateProofResponse { valid: false });
    }

    let inclusion_merkle_root = milestone.inclusion_merkle_root;

    // todo!("verify the contained milestone signatures");

    let merkle_hasher = MerkleTreeHasher::<Blake2b256>::new();

    Ok(ValidateProofResponse {
        valid: merkle_hasher.validate_proof(proof)?,
    })
}
