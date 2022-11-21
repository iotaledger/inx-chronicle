// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use hyper::StatusCode;
use iota_types::block::payload::milestone::MilestoneValidationError;
use thiserror::Error;

use crate::api::error::{impl_internal_error, ErrorStatus};

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

impl ErrorStatus for RequestError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum CorruptStateError {
    #[error("No milestone cone in the database")]
    NoMilestoneCone,
    #[error("Error decoding public key")]
    DecodePublicKey,
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

impl_internal_error!(CorruptStateError, CreateProofError);
