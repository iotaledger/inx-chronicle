// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::array::TryFromSliceError;

use thiserror::Error;

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BeeError(#[from] bee_block_stardust::Error),
    #[error("failed to convert to model type: {0}")]
    DtoEncodingFailed(#[from] TryFromSliceError),
}
