// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use chronicle::runtime::actor::Actor;

/// An event which will spawn a supervised actor
#[derive(Debug)]
pub(crate) struct SpawnRegistryActor<A: Actor> {
    pub actor: A,
}

impl<A: Actor> SpawnRegistryActor<A> {
    /// Creates a new [`SpawnRegistryActor`] event.
    pub fn new(actor: A) -> Self {
        Self { actor }
    }
}
