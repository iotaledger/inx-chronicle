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

use super::{
    merkle::{CreateAuditPathError, MerkleAuditPath, MerkleHasher},
    responses::{CreateProofResponse, ValidateProofResponse},
    CorruptStateError as PoiCorruptStateError, RequestError as PoiRequestError,
};
use crate::api::{
    error::{CorruptStateError, MissingError, RequestError},
    router::Router,
    ApiError, ApiResult,
};

pub fn routes() -> Router {
    Router::new()
        .route("/create/:block_id", get(create_proof))
        .route("/validate", post(validate_proof))
}

async fn create_proof(database: Extension<MongoDb>, Path(block_id): Path<String>) -> ApiResult<CreateProofResponse> {
    let block_id = BlockId::from_str(&block_id)?;
    let block_collection = database.collection::<BlockCollection>();

    // Ensure the corresponding block was referenced by a milestone.
    let block_metadata = block_collection
        .get_block_metadata(&block_id)
        .await?
        .ok_or(MissingError::NoResults)?;
    let referenced_index = block_metadata.referenced_by_milestone_index;
    if referenced_index == 0 {
        return Err(ApiError::from(PoiRequestError::BlockNotReferenced(block_id.to_hex())));
    }

    // Fetch the corresponding milestone cone in "White Flag" order.
    let block_ids = block_collection
        .get_pastcone_in_white_flag_order(referenced_index)
        .await?;
    if block_ids.is_empty() {
        return Err(ApiError::from(PoiCorruptStateError::NoMilestoneCone));
    }

    // Create the inclusion proof to return in the response.
    let proof = MerkleHasher::create_audit_path(&block_ids, &block_id)?;

    // Fetch the corresponding milestone to return in the response.
    let milestone_collection = database.collection::<MilestoneCollection>();
    let milestone = milestone_collection
        .get_milestone_payload(referenced_index)
        .await?
        .ok_or(MissingError::NoResults)?;

    let calculated_merkle_root = &*proof.hash();
    let expected_merkle_root = milestone.essence.inclusion_merkle_root;
    if calculated_merkle_root != expected_merkle_root {
        return Err(ApiError::from(CreateAuditPathError::MerkleRootMismatch {
            calculated_merkle_root: prefix_hex::encode(calculated_merkle_root),
            expected_merkle_root: prefix_hex::encode(expected_merkle_root),
        }));
    }

    // Fetch the corresponding block to return in the response.
    let block = block_collection
        .get_block(&block_id)
        .await?
        .ok_or(MissingError::NoResults)?;

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
        .map_err(|_| RequestError::PoI(PoiRequestError::MalformedJsonBlock))?;
    let milestone = iota_types::block::payload::milestone::MilestonePayload::try_from_dto_unverified(&milestone)
        .map_err(|_| RequestError::PoI(PoiRequestError::MalformedJsonMilestone))?;
    let proof = MerkleAuditPath::try_from(proof).map_err(|_| RequestError::PoI(PoiRequestError::MalformedJsonProof))?;

    let block_id = block.id().into();

    // Fetch the corresponding milestone to return in the response.
    let milestone_index = milestone.essence().index();

    // Fetch the node configuration.
    let update_collection = database.collection::<ConfigurationUpdateCollection>();
    let node_configuration = update_collection
        .get_node_configuration_for_ledger_index(milestone_index.into())
        .await?
        .ok_or(MissingError::NoResults)?
        .config;

    // Validate the given milestone.
    let public_key_count = node_configuration.milestone_public_key_count as usize;
    let key_ranges = node_configuration.milestone_key_ranges;
    let applicable_public_keys = get_valid_public_keys_for_index(key_ranges, milestone_index.into())?;

    if let Err(e) = milestone.validate(&applicable_public_keys, public_key_count) {
        Err(RequestError::PoI(PoiRequestError::InvalidMilestone(e)).into())
    } else {
        Ok(ValidateProofResponse {
            valid: proof.contains_block_id(&block_id) && *proof.hash() == **milestone.essence().inclusion_merkle_root(),
        })
    }
}

// The returned public keys must be hex strings without the `0x` prefix for the milestone validation to work.
#[allow(clippy::boxed_local)]
fn get_valid_public_keys_for_index(
    mut key_ranges: Box<[MilestoneKeyRange]>,
    index: MilestoneIndex,
) -> Result<Vec<String>, CorruptStateError> {
    key_ranges.sort();
    let mut public_keys = HashSet::with_capacity(key_ranges.len());
    for key_range in key_ranges.iter() {
        match (key_range.start, key_range.end) {
            (start, _) if start > index => break,
            (start, end) if index <= end || start == end => {
                let public_key_raw = prefix_hex::decode::<Vec<_>>(&key_range.public_key)
                    .map_err(|_| CorruptStateError::PoI(PoiCorruptStateError::DecodePublicKey))?;
                let public_key_hex = hex::encode(public_key_raw);
                public_keys.insert(public_key_hex);
            }
            (_, _) => continue,
        }
    }
    Ok(public_keys.into_iter().collect::<Vec<_>>())
}
