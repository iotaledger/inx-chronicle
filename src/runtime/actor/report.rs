// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{error::ActorError, Actor};

/// An actor exit report
pub type Report<A> = Result<A, ErrorReport<A>>;

/// A report that an actor finished running with an error
#[derive(Debug)]
pub struct ErrorReport<A: Actor> {
    /// The actor's state when it finished running
    pub state: A,
    /// The error that occurred
    pub error: ActorError,
}

impl<A: Actor> ErrorReport<A> {
    pub(crate) fn new(state: A, error: ActorError) -> Result<A, Self> {
        Err(Self { state, error })
    }
}
