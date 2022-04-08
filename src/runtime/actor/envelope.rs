// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, pin::Pin};

use async_trait::async_trait;

use super::{context::ActorContext, Actor};

/// Trait that allows handling events sent to an actor
#[async_trait]
pub trait HandleEvent<E: Send + Debug>: Actor + Sized {
    #[allow(missing_docs)]
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: E,
        data: &mut Self::Data,
    ) -> Result<(), Self::Error>;
}

/// A dynamic event that can be sent to an actor which implements `HandleEvent` for it
pub trait DynEvent<A: Actor>: Debug {
    #[allow(missing_docs)]
    fn handle<'c, 'a>(
        self: Box<Self>,
        cx: &'c mut ActorContext<A>,
        act: &'c mut A,
        data: &'c mut A::Data,
    ) -> Pin<Box<dyn core::future::Future<Output = Result<(), A::Error>> + Send + 'a>>
    where
        Self: 'a,
        'c: 'a;
}

impl<A, E: Send + Debug> DynEvent<A> for E
where
    A: HandleEvent<E>,
{
    fn handle<'c, 'a>(
        self: Box<Self>,
        cx: &'c mut ActorContext<A>,
        act: &'c mut A,
        data: &'c mut A::Data,
    ) -> Pin<Box<dyn core::future::Future<Output = Result<(), A::Error>> + Send + 'a>>
    where
        Self: 'a,
        'c: 'a,
    {
        act.handle_event(cx, *self, data)
    }
}

/// Convenience type for boxed dynamic events
pub type Envelope<A> = Box<dyn DynEvent<A> + Send + Sync>;
