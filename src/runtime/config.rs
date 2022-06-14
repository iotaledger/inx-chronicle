// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use super::actor::Actor;

/// Spawn configuration for an actor.
#[derive(Debug)]
pub struct SpawnConfig<A> {
    pub(crate) actor: A,
    pub(crate) config: SpawnConfigInner,
}

impl<A> SpawnConfig<A> {
    /// Creates a new spawn configuration.
    pub fn new(actor: A) -> Self {
        Self {
            actor,
            config: Default::default(),
        }
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

#[derive(Debug, Default)]
pub(crate) struct SpawnConfigInner {
    pub(crate) add_to_registry: bool,
}

impl SpawnConfigInner {
    /// Sets whether the actor's address should be added to the registry.
    pub(crate) fn set_add_to_registry(&mut self, add: bool) {
        self.add_to_registry = add;
    }
}

/// Helper methods for spawning actors.
pub trait ConfigureActor: Actor {
    /// Sets whether the actor's address should be added to the registry.
    fn with_registration(self, enable: bool) -> SpawnConfig<Self> {
        SpawnConfig::<Self>::new(self).with_registration(enable)
    }
}
impl<A: Actor> ConfigureActor for A {}
