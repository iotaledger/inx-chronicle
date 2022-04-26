// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{error::ActorError, Actor};

/// An actor exit report.
#[derive(Debug)]
pub enum Report<A: Actor> {
    /// Actor exited successfully.
    Success(SuccessReport<A>),
    /// Actor exited with an error.
    Error(ErrorReport<A>),
}

impl<A: Actor> Report<A> {
    /// Gets the actor.
    pub fn actor(&self) -> &A {
        match self {
            Report::Success(success) => success.actor(),
            Report::Error(error) => error.actor(),
        }
    }

    /// Takes the actor, consuming the report.
    pub fn take_actor(self) -> A {
        match self {
            Report::Success(success) => success.take_actor(),
            Report::Error(error) => error.take_actor(),
        }
    }

    /// Gets the internal state, if any was created. No state indicates that the init method never completed.
    pub fn internal_state(&self) -> Option<&A::State> {
        match self {
            Report::Success(success) => success.internal_state(),
            Report::Error(error) => error.internal_state(),
        }
    }

    /// Takes the internal state, if any was created. No state indicates that the init method never completed.
    pub fn take_internal_state(self) -> Option<A::State> {
        match self {
            Report::Success(success) => success.take_internal_state(),
            Report::Error(error) => error.take_internal_state(),
        }
    }

    /// Gets the error, if any.
    pub fn error(&self) -> Option<&ActorError<A>> {
        match self {
            Report::Success(_) => None,
            Report::Error(error) => Some(error.error()),
        }
    }

    /// Takes the error, if any.
    pub fn take_error(self) -> Option<ActorError<A>> {
        match self {
            Report::Success(_) => None,
            Report::Error(error) => Some(error.take_error()),
        }
    }
}

/// A report that an actor finished running with an error
#[derive(Debug)]
pub struct SuccessReport<A: Actor> {
    /// The actor's external state when it finished running
    pub actor: A,
    /// The actor's internal state when it finished running
    pub internal_state: Option<A::State>,
}

impl<A: Actor> SuccessReport<A> {
    pub(crate) fn new(actor: A, internal_state: Option<A::State>) -> Self {
        Self { actor, internal_state }
    }

    /// Gets the actor.
    pub fn actor(&self) -> &A {
        &self.actor
    }

    /// Takes the actor, consuming the report.
    pub fn take_actor(self) -> A {
        self.actor
    }

    /// Gets the internal state, if any was created. No state indicates that the init method never completed.
    pub fn internal_state(&self) -> Option<&A::State> {
        self.internal_state.as_ref()
    }

    /// Takes the internal state, if any was created. No state indicates that the init method never completed.
    pub fn take_internal_state(self) -> Option<A::State> {
        self.internal_state
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
    pub(crate) fn new(actor: A, internal_state: Option<A::State>, error: ActorError<A>) -> Self {
        Self {
            actor,
            internal_state,
            error,
        }
    }

    /// Gets the actor.
    pub fn actor(&self) -> &A {
        &self.actor
    }

    /// Takes the actor, consuming the report.
    pub fn take_actor(self) -> A {
        self.actor
    }

    /// Gets the internal state, if any was created. No state indicates that the init method never completed.
    pub fn internal_state(&self) -> Option<&A::State> {
        self.internal_state.as_ref()
    }

    /// Takes the internal state, if any was created. No state indicates that the init method never completed.
    pub fn take_internal_state(self) -> Option<A::State> {
        self.internal_state
    }

    /// Gets the error that occurred.
    pub fn error(&self) -> &ActorError<A> {
        &self.error
    }

    /// Takes the error that occurred.
    pub fn take_error(self) -> ActorError<A> {
        self.error
    }
}
