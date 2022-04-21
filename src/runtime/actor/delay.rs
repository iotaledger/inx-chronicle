// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, time::Instant};

use async_trait::async_trait;

use super::{
    event::{DynEvent, HandleEvent},
    Actor,
};

/// A wrapper that can be used to delay an event until a specified time.
#[derive(Debug)]
pub struct Delay<E> {
    /// The time to delay the event until.
    pub until: Instant,
    /// The event to delay.
    pub event: E,
}

impl<E> Delay<E> {
    /// Create a new [`Delay`] wrapper.
    pub fn new(event: E, until: Instant) -> Self {
        Self { event, until }
    }
}

#[async_trait]
impl<A, E> HandleEvent<Delay<E>> for A
where
    A: Actor,
    E: 'static + Send + Sync + Debug + DynEvent<A>,
{
    async fn handle_event(
        &mut self,
        cx: &mut super::context::ActorContext<Self>,
        event: Delay<E>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if Instant::now() >= event.until {
            cx.delay(event.event, None).unwrap();
        } else {
            cx.delay(event.event, event.until).unwrap();
        }
        Ok(())
    }
}
