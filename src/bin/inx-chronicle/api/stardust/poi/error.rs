// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
#[allow(missing_docs, clippy::enum_variant_names)]
pub enum PoIError {
    #[error("Invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("Invalid request: {0}")]
    InvalidRequest(&'static str),
    #[error("Invalid proof: {0}")]
    InvalidProof(&'static str),
}
