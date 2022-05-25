// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use thiserror::Error;

use super::{event::Envelope, sender::CloneSender, Actor};
use crate::runtime::{registry::ScopeId, scope::ScopeView};

/// Error sending a block to an actor
#[derive(Error, Debug)]
#[error("Error sending block to actor: {0}")]
pub struct SendError(String);

#[allow(missing_docs)]
impl SendError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl<S: Into<String>> From<S> for SendError {
    fn from(msg: S) -> Self {
        Self::new(msg)
    }
}

/// An actor handle, used to send events.
pub struct Addr<A: Actor> {
    pub(crate) scope: ScopeView,
    pub(crate) sender: Box<dyn CloneSender<Envelope<A>>>,
}

impl<A: Actor> Addr<A> {
    pub(crate) fn new(scope: ScopeView, sender: impl CloneSender<Envelope<A>> + 'static) -> Self {
        Self {
            scope,
            sender: Box::new(sender) as _,
        }
    }

    /// Shuts down the actor. Use with care!
    pub fn shutdown(&self) {
        self.scope.shutdown();
    }

    /// Aborts the actor. Use with care!
    pub async fn abort(&self) {
        self.scope.abort().await;
    }

    /// Gets the scope id of the actor this handle represents.
    pub fn scope_id(&self) -> ScopeId {
        self.scope.id()
    }
}

impl<A: Actor> Clone for Addr<A> {
    fn clone(&self) -> Self {
        Self {
            scope: self.scope.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl<A: Actor> std::fmt::Debug for Addr<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Addr").field("scope", &self.scope).finish()
    }
}

/// An optional address, which allows sending events.
#[derive(Debug, Clone)]
pub struct OptionalAddr<A: Actor>(pub(crate) Option<Addr<A>>);

impl<A: Actor> From<Option<Addr<A>>> for OptionalAddr<A> {
    fn from(opt_addr: Option<Addr<A>>) -> Self {
        Self(opt_addr)
    }
}

impl<A: Actor> Deref for OptionalAddr<A> {
    type Target = Option<Addr<A>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
