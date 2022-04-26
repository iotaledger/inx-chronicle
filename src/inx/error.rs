// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

// TODO: Consider moving this to the `iotaledger/inx` repository.

use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum InxError {
    #[error("failed to establish connection: {0}")]
    ConnectionError(tonic::transport::Error),
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error(transparent)]
    ParsingAddressFailed(#[from] url::ParseError),
    #[error(transparent)]
    TransportFailed(#[from] tonic::transport::Error),
}
