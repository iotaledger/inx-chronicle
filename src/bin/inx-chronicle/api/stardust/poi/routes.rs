// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashSet, str::FromStr};

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
    types::{node::MilestoneKeyRange, stardust::block::BlockId, tangle::MilestoneIndex},
};
use crypto::hashes::blake2b::Blake2b256;

use super::{
    error::PoIError,
    merkle_hasher::MerkleHasher,
    merkle_proof::MerkleProof,
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
    let proof = MerkleHasher::create_proof::<Blake2b256>(&block_ids, &block_id)?;

    // Fetch the corresponding milestone to return in the response.
    let milestone_collection = database.collection::<MilestoneCollection>();
    let milestone = milestone_collection
        .get_milestone_payload(referenced_index)
        .await?
        .ok_or(ApiError::NoResults)?;

    let inclusion_merkle_root = milestone.essence.inclusion_merkle_root;
    if *proof.hash() != inclusion_merkle_root {
        return Err(ApiError::PoI(PoIError::CreateProofError(block_id.to_hex())));
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
        return Err(ApiError::PoI(PoIError::InvalidRequest(
            "block not referenced by given milestone",
        )));
    }

    // Fetch the node configuration.
    let update_collection = database.collection::<ConfigurationUpdateCollection>();
    let node_configuration = update_collection
        .get_node_configuration_for_ledger_index(milestone_index)
        .await?
        .ok_or(ApiError::NoResults)?
        .config;

    // Validate the given milestone.
    let public_key_count = node_configuration.milestone_public_key_count as usize;
    let key_ranges = node_configuration.milestone_key_ranges;
    let applicable_public_keys = get_valid_public_keys_for_index(key_ranges, milestone_index);

    if let Err(e) = milestone.validate(&applicable_public_keys, public_key_count) {
        Err(ApiError::PoI(PoIError::InvalidMilestone(e)))
    } else {
        Ok(ValidateProofResponse {
            valid: proof.contains_block_id(&block_id) && *proof.hash() == **milestone.essence().inclusion_merkle_root(),
        })
    }
}

// The returned public keys must be hex strings without the `0x` prefix for the milestone validation to work.
#[allow(clippy::boxed_local)]
fn get_valid_public_keys_for_index(mut key_ranges: Box<[MilestoneKeyRange]>, index: MilestoneIndex) -> Vec<String> {
    key_ranges.sort();
    let mut public_keys = HashSet::with_capacity(key_ranges.len());
    for key_range in key_ranges.iter() {
        match (key_range.start, key_range.end) {
            (start, _) if start > index => break,
            (start, end) if index <= end || start == end => {
                // Panic: should never fail
                let public_key_raw = prefix_hex::decode::<Vec<_>>(&key_range.public_key).unwrap();
                let public_key_hex = hex::encode(public_key_raw);
                public_keys.insert(public_key_hex);
            }
            (_, _) => continue,
        }
    }
    public_keys.into_iter().collect::<Vec<_>>()
}
