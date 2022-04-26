// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use super::{
    event::{DynEvent, Envelope},
    Actor,
};
use crate::runtime::{error::RuntimeError, registry::ScopeId, scope::ScopeView};

/// Error sending a message to an actor
#[derive(Error, Debug)]
#[error("Error sending message to actor: {0}")]
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
#[derive(Debug)]
pub struct Addr<A: Actor> {
    pub(crate) scope: ScopeView,
    pub(crate) sender: UnboundedSender<Envelope<A>>,
}

impl<A: Actor> Addr<A> {
    pub(crate) fn new(scope: ScopeView, sender: UnboundedSender<Envelope<A>>) -> Self {
        Self { scope, sender }
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

    /// Sends a message to the actor
    pub fn send<E: 'static + DynEvent<A> + Send + Sync>(&self, event: E) -> Result<(), RuntimeError>
    where
        Self: Sized,
    {
        self.sender
            .send(Box::new(event))
            .map_err(|_| RuntimeError::SendError("Failed to send event".into()))
    }

    /// Returns whether the actor's event channel is closed.
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
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
