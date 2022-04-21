// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, sync::Arc};

use thiserror::Error;

use super::{actor::addr::SendError, registry::ScopeId};

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Scope {0:x} aborted")]
    AbortedScope(ScopeId),
    #[error("Actor exited with error: {0}")]
    ActorError(Arc<dyn Error + Send + Sync>),
    #[error("Dependency notification canceled")]
    CanceledDepNotification,
    #[error("Invalid scope")]
    InvalidScope,
    #[error("Missing dependency: {0}")]
    MissingDependency(String),
    #[error("Error launching scope: {0}")]
    ScopeLaunchError(Box<dyn Error + Send + Sync>),
    #[error(transparent)]
    SendError(#[from] SendError),
    #[error("Task exited with error: {0}")]
    TaskError(Box<dyn Error + Send + Sync>),
}
