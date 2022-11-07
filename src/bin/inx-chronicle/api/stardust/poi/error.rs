// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum PoIError {
    #[error("Invalid request: {0}")]
    InvalidRequest(&'static str),
    #[error("Invalid proof for: {0}")]
    InvalidProof(String),
}
