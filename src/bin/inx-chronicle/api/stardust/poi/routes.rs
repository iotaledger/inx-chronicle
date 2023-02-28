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
    types::{
        node::MilestoneKeyRange,
        stardust::tangle::{block::BlockId, milestone::MilestoneIndex},
    },
};

use super::{
    error as poi,
    merkle_proof::{MerkleAuditPath, MerkleProof},
    responses::{CreateProofResponse, ValidateProofResponse},
};
use crate::api::{
    error::{CorruptStateError, MissingError, RequestError},
    router::Router,
    ApiResult,
};

pub fn routes() -> Router {
    Router::new()
        .route(
            "/referenced-block/create/:block_id",
            get(create_proof_for_referenced_blocks),
        )
        .route("/referenced-block/validate", post(validate_proof_for_referenced_blocks))
        .route("/applied-block/create/:block_id", get(create_proof_for_applied_blocks))
        .route("/applied-block/validate", post(validate_proof_for_applied_blocks))
}

async fn create_proof_for_referenced_blocks(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
) -> ApiResult<CreateProofResponse> {
    let block_id = BlockId::from_str(&block_id)?;
    let block_collection = database.collection::<BlockCollection>();
    let milestone_collection = database.collection::<MilestoneCollection>();

    // Check if the metadata for that block exists.
    let block_metadata = block_collection
        .get_block_metadata(&block_id)
        .await?
        .ok_or(MissingError::NoResults)?;

    // Check whether the block was referenced by a milestone.
    let referenced_index = block_metadata.referenced_by_milestone_index;
    if referenced_index == 0 {
        return Err(RequestError::PoI(poi::RequestError::BlockNotReferenced(block_id.to_hex())).into());
    }

    // Fetch the block to return in the response.
    let block = block_collection
        .get_block(&block_id)
        .await?
        .ok_or(MissingError::NoResults)?;

    // Fetch the referencing milestone payload.
    let milestone_payload = milestone_collection
        .get_milestone_payload(referenced_index)
        .await?
        .ok_or(MissingError::NoResults)?;

    // Fetch the referenced block ids in "White Flag" order, and make sure they contain the block.
    let referenced_block_ids = block_collection
        .get_referenced_blocks_in_white_flag_order(referenced_index)
        .await?;
    if referenced_block_ids.is_empty() {
        return Err(CorruptStateError::PoI(poi::CorruptStateError::NoMilestoneCone).into());
    } else if !referenced_block_ids.contains(&block_id) {
        return Err(CorruptStateError::PoI(poi::CorruptStateError::IncompleteMilestoneCone).into());
    }

    // Create the Merkle audit path for the given block against that ordered set of referenced block ids.
    let merkle_audit_path = MerkleProof::create_audit_path(&referenced_block_ids, &block_id)
        .map_err(|e| CorruptStateError::PoI(poi::CorruptStateError::CreateProof(e)))?;

    // Ensure that the generated audit path is correct by comparing its hash with the one stored in the milestone.
    let calculated_merkle_root = merkle_audit_path.hash();
    let expected_merkle_root = milestone_payload.essence.inclusion_merkle_root;
    if calculated_merkle_root.as_slice() != expected_merkle_root {
        return Err(CorruptStateError::PoI(poi::CorruptStateError::CreateProof(
            poi::CreateProofError::MerkleRootMismatch {
                calculated_merkle_root: prefix_hex::encode(calculated_merkle_root.as_slice()),
                expected_merkle_root: prefix_hex::encode(expected_merkle_root),
            },
        ))
        .into());
    }

    Ok(CreateProofResponse {
        milestone: milestone_payload.into(),
        block: block.into(),
        audit_path: merkle_audit_path.into(),
    })
}

async fn validate_proof_for_referenced_blocks(
    database: Extension<MongoDb>,
    Json(CreateProofResponse {
        milestone,
        block,
        audit_path: merkle_path,
    }): Json<CreateProofResponse>,
) -> ApiResult<ValidateProofResponse> {
    // Extract block, milestone, and audit path.
    let block = iota_types::block::Block::try_from_dto_unverified(&block)
        .map_err(|_| RequestError::PoI(poi::RequestError::MalformedJsonBlock))?;
    let block_id = block.id().into();
    let milestone = iota_types::block::payload::milestone::MilestonePayload::try_from_dto_unverified(&milestone)
        .map_err(|_| RequestError::PoI(poi::RequestError::MalformedJsonMilestone))?;
    let milestone_index = milestone.essence().index();
    let proof = MerkleAuditPath::try_from(merkle_path)
        .map_err(|_| RequestError::PoI(poi::RequestError::MalformedJsonAuditPath))?;

    // Fetch public keys to verify the milestone signatures.
    let update_collection = database.collection::<ConfigurationUpdateCollection>();
    let node_configuration = update_collection
        .get_node_configuration_for_ledger_index(milestone_index.into())
        .await?
        .ok_or(MissingError::NoResults)?
        .config;
    let public_key_count = node_configuration.milestone_public_key_count as usize;
    let key_ranges = node_configuration.milestone_key_ranges;
    let applicable_public_keys = get_valid_public_keys_for_index(key_ranges, milestone_index.into())?;

    // Validate the given milestone.
    if let Err(e) = milestone.validate(&applicable_public_keys, public_key_count) {
        Err(RequestError::PoI(poi::RequestError::InvalidMilestone(e)).into())
    } else {
        Ok(ValidateProofResponse {
            valid: proof.contains_block_id(&block_id) && *proof.hash() == **milestone.essence().inclusion_merkle_root(),
        })
    }
}

async fn create_proof_for_applied_blocks(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
) -> ApiResult<CreateProofResponse> {
    let block_id = BlockId::from_str(&block_id)?;
    let block_collection = database.collection::<BlockCollection>();
    let milestone_collection = database.collection::<MilestoneCollection>();

    // Check if the metadata for that block exists.
    let block_metadata = block_collection
        .get_block_metadata(&block_id)
        .await?
        .ok_or(MissingError::NoResults)?;

    // Check whether the block was referenced by a milestone, and whether it caused a ledger mutation.
    let referenced_index = block_metadata.referenced_by_milestone_index;
    if referenced_index == 0 {
        return Err(RequestError::PoI(poi::RequestError::BlockNotReferenced(block_id.to_hex())).into());
    } else if block_metadata.inclusion_state != chronicle::types::stardust::ledger::LedgerInclusionState::Included {
        return Err(RequestError::PoI(poi::RequestError::BlockNotApplied(block_id.to_hex())).into());
    }

    // Fetch the block to return in the response.
    let block = block_collection
        .get_block(&block_id)
        .await?
        .ok_or(MissingError::NoResults)?;

    // Fetch the referencing milestone.
    let milestone = milestone_collection
        .get_milestone_payload(referenced_index)
        .await?
        .ok_or(MissingError::NoResults)?;

    // Fetch the referenced and applied block ids in "White Flag" order, and make sure they contain the block.
    let applied_block_ids = block_collection
        .get_applied_blocks_in_white_flag_order(referenced_index)
        .await?;
    if !applied_block_ids.contains(&block_id) {
        return Err(RequestError::PoI(poi::RequestError::BlockNotApplied(block_id.to_hex())).into());
    }

    // Create the Merkle audit path for the given block against that ordered set of referenced and applied block ids.
    let merkle_audit_path = MerkleProof::create_audit_path(&applied_block_ids, &block_id)
        .map_err(|e| CorruptStateError::PoI(poi::CorruptStateError::CreateProof(e)))?;

    // Ensure that the generated audit path is correct by comparing its hash with the one stored in the milestone.
    let calculated_merkle_root = merkle_audit_path.hash();
    let expected_merkle_root = milestone.essence.applied_merkle_root;
    if calculated_merkle_root.as_slice() != expected_merkle_root {
        return Err(CorruptStateError::PoI(poi::CorruptStateError::CreateProof(
            poi::CreateProofError::MerkleRootMismatch {
                calculated_merkle_root: prefix_hex::encode(calculated_merkle_root.as_slice()),
                expected_merkle_root: prefix_hex::encode(expected_merkle_root),
            },
        ))
        .into());
    }

    Ok(CreateProofResponse {
        milestone: milestone.into(),
        block: block.into(),
        audit_path: merkle_audit_path.into(),
    })
}

async fn validate_proof_for_applied_blocks(
    database: Extension<MongoDb>,
    Json(CreateProofResponse {
        milestone,
        block,
        audit_path,
    }): Json<CreateProofResponse>,
) -> ApiResult<ValidateProofResponse> {
    // Extract block, milestone, and audit path.
    let block = iota_types::block::Block::try_from_dto_unverified(&block)
        .map_err(|_| RequestError::PoI(poi::RequestError::MalformedJsonBlock))?;
    let block_id = block.id().into();
    let milestone = iota_types::block::payload::milestone::MilestonePayload::try_from_dto_unverified(&milestone)
        .map_err(|_| RequestError::PoI(poi::RequestError::MalformedJsonMilestone))?;
    let milestone_index = milestone.essence().index();
    let audit_path = MerkleAuditPath::try_from(audit_path)
        .map_err(|_| RequestError::PoI(poi::RequestError::MalformedJsonAuditPath))?;

    // Fetch public keys to verify the milestone signatures.
    let update_collection = database.collection::<ConfigurationUpdateCollection>();
    let node_configuration = update_collection
        .get_node_configuration_for_ledger_index(milestone_index.into())
        .await?
        .ok_or(MissingError::NoResults)?
        .config;
    let public_key_count = node_configuration.milestone_public_key_count as usize;
    let key_ranges = node_configuration.milestone_key_ranges;
    let applicable_public_keys = get_valid_public_keys_for_index(key_ranges, milestone_index.into())?;

    // Validate the given milestone.
    if let Err(e) = milestone.validate(&applicable_public_keys, public_key_count) {
        Err(RequestError::PoI(poi::RequestError::InvalidMilestone(e)).into())
    } else {
        Ok(ValidateProofResponse {
            valid: audit_path.contains_block_id(&block_id)
                && *audit_path.hash() == **milestone.essence().applied_merkle_root(),
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
                    .map_err(|_| CorruptStateError::PoI(poi::CorruptStateError::DecodePublicKey))?;
                let public_key_hex = hex::encode(public_key_raw);
                public_keys.insert(public_key_hex);
            }
            (_, _) => continue,
        }
    }
    Ok(public_keys.into_iter().collect::<Vec<_>>())
}
