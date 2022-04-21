// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use thiserror::Error;

use super::Actor;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum ActorError<A: Actor> {
    #[error("Actor aborted")]
    Aborted,
    #[error("Actor panicked")]
    Panic,
    #[error("Actor error: {0:?}")]
    Result(Arc<A::Error>),
}
