// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use crate::api::error::impl_internal_error;

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

impl_internal_error!(CreateProofError);
