// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use super::{
    envelope::{DynEvent, Envelope},
    Actor,
};
use crate::runtime::{registry::ScopeId, scope::ScopeView};

/// Error sending a message to an actor
#[derive(Error, Debug)]
#[error("Error sending message to actor: {0}")]
pub struct SendError(String);

impl SendError {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self(msg.into())
    }
}

impl<S: Into<String>> From<S> for SendError {
    fn from(msg: S) -> Self {
        Self::new(msg)
    }
}

/// An actor handle, used to send events
#[derive(Debug)]
pub struct Act<A: Actor> {
    pub(crate) scope: ScopeView,
    pub(crate) sender: UnboundedSender<Envelope<A>>,
}

impl<A: Actor> Act<A> {
    pub(crate) fn new(scope: ScopeView, sender: UnboundedSender<Envelope<A>>) -> Self {
        Self { scope, sender }
    }

    /// Shut down the actor with this handle. Use with care!
    pub async fn shutdown(&self) {
        self.scope.shutdown().await;
    }

    /// Abort the actor with this handle. Use with care!
    pub async fn abort(&self) {
        self.scope.abort().await;
    }

    /// Get the scope id of the actor this handle represents
    pub fn scope_id(&self) -> ScopeId {
        self.scope.id()
    }

    pub fn send<E: 'static + DynEvent<A> + Send + Sync>(&self, event: E) -> Result<(), SendError>
    where
        Self: Sized,
    {
        self.sender
            .send(Box::new(event))
            .map_err(|_| "Failed to send event".into())
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl<A: 'static + Actor> Clone for Act<A> {
    fn clone(&self) -> Self {
        Self {
            scope: self.scope.clone(),
            sender: self.sender.clone(),
        }
    }
}
