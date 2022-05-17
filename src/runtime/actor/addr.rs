// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use thiserror::Error;

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
pub struct Addr<A: Actor> {
    pub(crate) scope: ScopeView,
    #[cfg(not(feature = "metrics"))]
    pub(crate) sender: tokio::sync::mpsc::UnboundedSender<Envelope<A>>,
    #[cfg(feature = "metrics")]
    pub(crate) sender: bee_metrics::metrics::sync::mpsc::UnboundedSender<Envelope<A>>,
}

impl<A: Actor> Addr<A> {
    #[cfg(not(feature = "metrics"))]
    pub(crate) fn new(scope: ScopeView, sender: tokio::sync::mpsc::UnboundedSender<Envelope<A>>) -> Self {
        Self { scope, sender }
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn new(
        scope: ScopeView,
        sender: bee_metrics::metrics::sync::mpsc::UnboundedSender<Envelope<A>>,
    ) -> Self {
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
    pub fn send<E: 'static + DynEvent<A>>(&self, event: E) -> Result<(), RuntimeError>
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

impl<A: Actor> std::fmt::Debug for Addr<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Addr").field("scope", &self.scope).finish()
    }
}

/// An optional address, which allows sending events.
#[derive(Debug, Clone)]
pub struct OptionalAddr<A: Actor>(Option<Addr<A>>);

impl<A: Actor> OptionalAddr<A> {
    /// Sends an event if the address exists. Returns an error if the address is not set.
    pub fn send<E>(&self, event: E) -> Result<(), RuntimeError>
    where
        A: 'static + Actor,
        E: 'static + DynEvent<A>,
    {
        self.0
            .as_ref()
            .ok_or_else(|| SendError::new(format!("No open address for {}", std::any::type_name::<A>())))?
            .send(event)
    }
}

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
