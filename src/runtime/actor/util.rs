// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, time::Duration};

use async_trait::async_trait;

use super::{
    context::ActorContext,
    event::{DynEvent, HandleEvent},
    report::Report,
    Actor,
};

/// A wrapper that can be used to delay an event until a specified time.
#[derive(Debug)]
pub struct DelayedEvent<E> {
    /// The time to delay the event until.
    pub delay: Duration,
    /// The event to delay.
    pub event: E,
}

impl<E> DelayedEvent<E> {
    /// Create a new [`DelayedEvent`] wrapper.
    pub fn new(event: E, delay: Duration) -> Self {
        Self { event, delay }
    }
}

#[async_trait]
impl<A, E> HandleEvent<DelayedEvent<E>> for A
where
    A: 'static + Actor,
    E: 'static + Send + Sync + Debug + DynEvent<A>,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: DelayedEvent<E>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let handle = cx.handle().clone();
        tokio::spawn(async move {
            tokio::time::sleep(event.delay).await;
            handle.send(event.event).unwrap();
        });
        Ok(())
    }
}

/// An event which will spawn a supervised actor.
#[derive(Debug)]
pub struct SpawnActor<A: Actor> {
    actor: A,
}

impl<A: Actor> SpawnActor<A> {
    /// Creates a new [`SpawnActor`] event.
    pub fn new(actor: A) -> Self {
        Self { actor }
    }
}

#[async_trait]
impl<T, A> HandleEvent<SpawnActor<A>> for T
where
    T: 'static + Actor + HandleEvent<Report<A>>,
    A: 'static + Actor + Debug + Send + Sync,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: SpawnActor<A>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        cx.spawn_actor_supervised(event.actor).await;
        Ok(())
    }
}
