// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, sync::Arc};

use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum ActorError {
    #[error("Actor error: {0:?}")]
    Result(Arc<Box<dyn Error + Send + Sync>>),
    #[error("Actor panicked")]
    Panic,
    #[error("Actor aborted")]
    Aborted,
}
