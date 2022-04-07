// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, sync::Arc};

use thiserror::Error;

use super::{actor::handle::SendError, registry::ScopeId};

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Invalid scope")]
    InvalidScope,
    #[error("Dependency notification canceled")]
    CanceledDepNotification,
    #[error("Missing dependency: {0}")]
    MissingDependency(String),
    #[error("Error launching scope: {0}")]
    ScopeLaunchError(Box<dyn Error + Send + Sync>),
    #[error("Scope {0:x} aborted")]
    AbortedScope(ScopeId),
    #[error("Duplicate actor {0} in scope {1:x}")]
    DuplicateActor(String, ScopeId),
    #[error("Actor exited with error: {0}")]
    ActorError(Arc<Box<dyn Error + Send + Sync>>),
    #[error(transparent)]
    SendError(#[from] SendError),
}
