// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing the actor address handle.
pub mod addr;
/// Module containing the actor context.
pub mod context;
/// Module containing actor error types.
pub mod error;
/// Module containing event types.
pub mod event;
/// Module containing actor exit report types.
pub mod report;
/// Module containing utilities.
pub mod util;

use std::{borrow::Cow, error::Error};

use async_trait::async_trait;
use context::ActorContext;
use futures::StreamExt;

/// The actor trait, which defines a task that is managed by the runtime.
#[async_trait]
pub trait Actor: Send + Sync + Sized {
    /// Custom data that is passed to all actor methods.
    type State: Send;
    /// Custom error type that is returned by all actor methods.
    type Error: Error + Send;

    /// Set this actor's name, primarily for debugging purposes.
    fn name(&self) -> Cow<'static, str> {
        std::any::type_name::<Self>().into()
    }

    /// Start the actor, and create the internal state.
    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error>;

    /// Run the actor event loop
    async fn run(&mut self, cx: &mut ActorContext<Self>, state: &mut Self::State) -> Result<(), Self::Error> {
        while let Some(evt) = cx.inbox().next().await {
            // Handle the event
            evt.handle(cx, self, state).await?;
        }
        log::debug!("{} exited event loop ({})", self.name(), cx.id());
        Ok(())
    }

    /// Handle any processing that needs to happen on shutdown
    async fn shutdown(&mut self, cx: &mut ActorContext<Self>, _state: &mut Self::State) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());
        Ok(())
    }
}
