// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

// TODO: Consider moving this to the `iotaledger/inx` repository.

use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum InxError {
    #[error("tonic transport failed")]
    TransportFailed, // TODO: Add actual error as a field
    #[error("missing field: `{0}`")]
    MissingField(&'static str),
    #[error("invalid field: `{0}`")]
    InvalidField(&'static str),
}
