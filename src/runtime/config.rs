// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use futures::{Stream, StreamExt};

use super::{
    actor::{
        event::{DynEvent, Envelope},
        Actor,
    },
    merge::Merge,
};

/// Spawn configuration for an actor.
#[derive(Debug)]
pub struct SpawnConfig<A> {
    pub(crate) actor: A,
    pub(crate) config: SpawnConfigInner<A>,
}

impl<A> SpawnConfig<A> {
    /// Creates a new spawn configuration.
    pub fn new(actor: A) -> Self {
        Self {
            actor,
            config: Default::default(),
        }
    }

    /// Merges a custom stream in addition to the event stream.
    pub fn with_stream<S, E>(mut self, stream: S) -> Self
    where
        A: Actor,
        S: 'static + Stream<Item = E> + Unpin + Send,
        E: 'static + DynEvent<A>,
    {
        self.config.add_stream(stream);
        self
    }

    /// Sets whether the actor's address should be added to the registry.
    pub fn with_registration(mut self, enable: bool) -> Self {
        self.config.set_add_to_registry(enable);
        self
    }
}

impl<A: Actor> From<A> for SpawnConfig<A> {
    fn from(actor: A) -> Self {
        Self::new(actor)
    }
}

pub(crate) struct SpawnConfigInner<A> {
    pub(crate) streams: Option<Merge<Envelope<A>>>,
    pub(crate) add_to_registry: bool,
}

impl<A: Debug> Debug for SpawnConfigInner<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpawnConfigInner")
            .field(
                "streams",
                &self
                    .streams
                    .as_ref()
                    .map(|_| std::any::type_name::<Merge<Envelope<A>>>()),
            )
            .field("add_to_registry", &self.add_to_registry)
            .finish()
    }
}

impl<A> Default for SpawnConfigInner<A> {
    fn default() -> Self {
        Self {
            streams: None,
            add_to_registry: true,
        }
    }
}

impl<A> SpawnConfigInner<A> {
    /// Merges a custom stream in addition to the event stream.
    pub(crate) fn add_stream<S, E>(&mut self, stream: S)
    where
        A: Actor,
        S: 'static + Stream<Item = E> + Unpin + Send,
        E: 'static + DynEvent<A>,
    {
        match self.streams.as_mut() {
            Some(streams) => {
                streams.merge(stream.map(|e| Box::new(e) as Envelope<A>));
            }
            None => {
                self.streams = Some(Merge::new(stream.map(|e| Box::new(e) as Envelope<A>)));
            }
        }
    }

    /// Sets whether the actor's address should be added to the registry.
    pub(crate) fn set_add_to_registry(&mut self, add: bool) {
        self.add_to_registry = add;
    }
}

/// Helper methods for spawning actors.
pub trait ConfigureActor: Actor {
    /// Merges acustom stream in addition to the event stream.
    fn with_stream<S, E>(self, stream: S) -> SpawnConfig<Self>
    where
        S: 'static + Stream<Item = E> + Unpin + Send,
        E: 'static + DynEvent<Self>,
    {
        SpawnConfig::<Self>::new(self).with_stream(stream)
    }

    /// Sets whether the actor's address should be added to the registry.
    fn with_registration(self, enable: bool) -> SpawnConfig<Self> {
        SpawnConfig::<Self>::new(self).with_registration(enable)
    }
}
impl<A: Actor> ConfigureActor for A {}
