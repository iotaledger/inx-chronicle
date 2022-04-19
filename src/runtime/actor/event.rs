// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, pin::Pin};

use async_trait::async_trait;
use futures::Stream;

use super::{context::ActorContext, Actor};

/// Trait that allows handling events sent to an actor.
#[async_trait]
pub trait HandleEvent<E: Send + Debug>: Actor + Sized {
    #[allow(missing_docs)]
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: E,
        state: &mut Self::State,
    ) -> Result<(), Self::Error>;
}

/// A dynamic event that can be sent to an actor which implements `HandleEvent` for it.
pub trait DynEvent<A: Actor>: Debug {
    #[allow(missing_docs)]
    fn handle<'a>(
        self: Box<Self>,
        cx: &'a mut ActorContext<A>,
        actor: &'a mut A,
        state: &'a mut A::State,
    ) -> Pin<Box<dyn core::future::Future<Output = Result<(), A::Error>> + Send + 'a>>
    where
        Self: 'a;
}

impl<A, E: Send + Debug> DynEvent<A> for E
where
    A: HandleEvent<E>,
{
    fn handle<'a>(
        self: Box<Self>,
        cx: &'a mut ActorContext<A>,
        actor: &'a mut A,
        state: &'a mut A::State,
    ) -> Pin<Box<dyn core::future::Future<Output = Result<(), A::Error>> + Send + 'a>>
    where
        Self: 'a,
    {
        actor.handle_event(cx, *self, state)
    }
}

/// Convenience type for boxed dynamic events.
pub type Envelope<A> = Box<dyn DynEvent<A> + Send + Sync>;
/// Convenience type for streams of dynamic events.
pub type EnvelopeStream<A> = Box<dyn Stream<Item = Envelope<A>> + Unpin + Send>;
