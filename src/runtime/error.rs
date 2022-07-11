// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use thiserror::Error;

use super::{actor::addr::SendError, registry::ScopeId};

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Scope {0:x} aborted")]
    AbortedScope(ScopeId),
    #[error("Actor exited with error: {0}")]
    ActorError(String),
    #[error("Error launching scope: {0}")]
    ScopeLaunchError(Box<dyn Error + Send + Sync>),
    #[error(transparent)]
    SendError(#[from] SendError),
}

/// Defines an error's log level.
pub trait ErrorLevel: Error {
    /// Returns the log level for this error.
    fn level(&self) -> log::Level {
        log::Level::Error
    }
}

impl ErrorLevel for RuntimeError {
    fn level(&self) -> log::Level {
        log::Level::Warn
    }
}

impl ErrorLevel for std::convert::Infallible {}
