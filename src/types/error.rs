// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::array::TryFromSliceError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BeeError(#[from] bee_message_stardust::Error),
    #[error("failed to convert to DTO type")]
    DtoEncodingFailed(#[from] TryFromSliceError),
}
