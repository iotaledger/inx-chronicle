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
use crate::runtime::{config::SpawnConfig, spawn_task, Sender};

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
    E: 'static + DynEvent<A>,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: DelayedEvent<E>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let handle = cx.handle().clone();
        spawn_task("delay event sleeper", async move {
            tokio::time::sleep(event.delay).await;
            handle.send(event.event).unwrap();
        });
        Ok(())
    }
}

/// An event which will spawn a supervised actor.
#[derive(Debug)]
pub struct SpawnActor<A: Actor> {
    actor: SpawnConfig<A>,
}

impl<A: Actor> SpawnActor<A> {
    /// Creates a new [`SpawnActor`] event.
    pub fn new<Cfg: Into<SpawnConfig<A>>>(actor: Cfg) -> Self {
        Self { actor: actor.into() }
    }
}

#[async_trait]
impl<T, A> HandleEvent<SpawnActor<A>> for T
where
    T: 'static + Actor + HandleEvent<Report<A>>,
    A: 'static + Actor,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: SpawnActor<A>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        cx.spawn_child(event.actor).await;
        Ok(())
    }
}

#[cfg(feature = "metrics")]
pub(crate) fn sanitize_metric_name(name: &str) -> String {
    name.chars()
        .filter_map(|c| match c {
            '<' => Some('_'),
            '_' | ':' => Some(c),
            c if c.is_whitespace() => Some('_'),
            c if c.is_ascii_alphanumeric() => Some(c),
            _ => None,
        })
        .collect::<String>()
}
