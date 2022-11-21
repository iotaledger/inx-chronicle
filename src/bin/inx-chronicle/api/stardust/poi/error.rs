// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::block::payload::milestone::MilestoneValidationError;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum RequestError {
    #[error("Invalid JSON representation of given block")]
    MalformedJsonBlock,
    #[error("Invalid JSON representation of given milestone")]
    MalformedJsonMilestone,
    #[error("Invalid JSON representation of given proof")]
    MalformedJsonProof,
    #[error("Block '{0}' not referenced")]
    BlockNotReferenced(String),
    #[error("Invalid milestone: {0:?}")]
    InvalidMilestone(MilestoneValidationError),
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum CorruptStateError {
    #[error("No milestone cone in the database")]
    NoMilestoneCone,
    #[error("Creating proof failed: {0}")]
    CreateProof(#[from] CreateProofError),
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum CreateProofError {
    #[error("Block '{0}' is not included in the given ordered list of blocks")]
    BlockNotIncluded(String),
    #[error("A proof cannot be created from {0} block ids")]
    InsufficientBlockIds(usize),
    #[error(
        "The calculated merkle root '{calculated_merkle_root}' does not match the expected: '{expected_merkle_root}'"
    )]
    MerkleRootMismatch {
        calculated_merkle_root: String,
        expected_merkle_root: String,
    },
}
