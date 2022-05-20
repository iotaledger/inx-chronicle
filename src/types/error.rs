// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::array::TryFromSliceError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error parsing from string: {0}")]
    ParseError(#[from] ParseError),
    #[error(transparent)]
    BeeError(#[from] bee_block_stardust::Error),
    #[error("failed to convert to DTO type")]
    DtoEncodingFailed(#[from] TryFromSliceError),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    BeeError(#[from] bee_block_stardust::Error),
}
