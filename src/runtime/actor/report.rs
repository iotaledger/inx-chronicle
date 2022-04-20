// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{error::ActorError, Actor};

/// An actor exit report
pub type Report<A> = Result<SuccessReport<A>, ErrorReport<A>>;

/// A report that an actor finished running with an error
#[derive(Debug)]
pub struct SuccessReport<A: Actor> {
    /// The actor's external state when it finished running
    pub actor: A,
    /// The actor's internal state when it finished running
    pub internal_state: Option<A::State>,
}

impl<A: Actor> SuccessReport<A> {
    pub(crate) fn new(actor: A, internal_state: Option<A::State>) -> Report<A> {
        Ok(Self { actor, internal_state })
    }
}

/// A report that an actor finished running with an error.
#[derive(Debug)]
pub struct ErrorReport<A: Actor> {
    /// The actor's external state when it finished running.
    pub actor: A,
    /// The actor's internal state when it finished running
    pub internal_state: Option<A::State>,
    /// The error that occurred
    pub error: ActorError<A>,
}

impl<A: Actor> ErrorReport<A> {
    pub(crate) fn new(actor: A, internal_state: Option<A::State>, error: ActorError<A>) -> Report<A> {
        Err(Self {
            actor,
            internal_state,
            error,
        })
    }
}
