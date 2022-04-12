// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{error::ActorError, Actor};

/// An actor exit report
pub type Report<A> = Result<SuccessReport<A>, ErrorReport<A>>;

/// A report that an actor finished running with an error
#[derive(Debug)]
pub struct SuccessReport<A: Actor> {
    /// The actor's state when it finished running
    pub state: A,
    /// The actor's data when it finished running
    pub data: Option<A::Data>,
}

impl<A: Actor> SuccessReport<A> {
    pub(crate) fn new(state: A, data: Option<A::Data>) -> Report<A> {
        Ok(Self { state, data })
    }
}

/// A report that an actor finished running with an error
#[derive(Debug)]
pub struct ErrorReport<A: Actor> {
    /// The actor's state when it finished running
    pub state: A,
    /// The actor's data when it finished running
    pub data: Option<A::Data>,
    /// The error that occurred
    pub error: ActorError,
}

impl<A: Actor> ErrorReport<A> {
    pub(crate) fn new(state: A, data: Option<A::Data>, error: ActorError) -> Report<A> {
        Err(Self { state, data, error })
    }
}
