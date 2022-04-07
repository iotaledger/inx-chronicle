// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, pin::Pin};

use async_trait::async_trait;

use super::{context::ActorContext, Actor};

#[async_trait]
pub trait HandleEvent<E: Send + Debug>: Actor + Sized {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: E,
        data: &mut Self::Data,
    ) -> Result<(), Self::Error>;
}

pub trait DynEvent<A: Actor>: Debug {
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

pub type Envelope<A> = Box<dyn DynEvent<A> + Send + Sync>;
