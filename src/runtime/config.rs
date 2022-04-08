// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, StreamExt};

use super::actor::{
    envelope::{DynEvent, Envelope},
    Actor,
};

/// Spawn configuration for an actor
pub struct SpawnConfig<A> {
    pub(crate) actor: A,
    pub(crate) stream: Option<Box<dyn Stream<Item = Envelope<A>> + Unpin + Send>>,
}

impl<A> SpawnConfig<A> {
    /// Create a new spawn configuration
    pub fn new(actor: A) -> Self {
        Self { actor, stream: None }
    }
}

impl<A> From<A> for SpawnConfig<A> {
    fn from(actor: A) -> Self {
        Self::new(actor)
    }
}

impl<A> SpawnConfig<A> {
    /// Use a custom stream in addition to the event stream
    pub fn with_stream<S, E>(self, stream: S) -> Self
    where
        A: Actor,
        S: 'static + Stream<Item = E> + Unpin + Send,
        E: 'static + DynEvent<A> + Send + Sync,
    {
        Self {
            actor: self.actor,
            stream: Some(Box::new(stream.map(|e| Box::new(e) as Envelope<A>))),
        }
    }
}

/// Helper methods for spawning actors
pub trait ConfigureActor: Actor {
    /// Use a custom stream in addition to the event stream
    fn with_stream<S, E>(self, stream: S) -> SpawnConfig<Self>
    where
        S: 'static + Stream<Item = E> + Unpin + Send,
        E: 'static + DynEvent<Self> + Send + Sync,
    {
        SpawnConfig::<Self>::new(self).with_stream(stream)
    }
}
impl<A: Actor> ConfigureActor for A {}
