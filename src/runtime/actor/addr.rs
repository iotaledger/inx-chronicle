// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use super::{
    event::{DynEvent, Envelope},
    Actor,
};
use crate::runtime::{error::RuntimeError, registry::ScopeId, scope::ScopeView};

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

    /// Sends a block to the actor
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

    /// Gets the inner optional address.
    pub fn into_inner(self) -> Option<Addr<A>> {
        self.0
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
