// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use async_trait::async_trait;
use futures::StreamExt;

use self::context::ActorContext;

pub mod context;
pub mod envelope;
pub mod error;
pub mod handle;
pub mod report;

#[async_trait]
pub trait Actor: Send + Sync + Sized {
    type Data: Send + Sync;
    type Error: Error + Send + Sync;

    /// Set this actor's name, primarily for debugging purposes
    fn name(&self) -> String {
        std::any::type_name::<Self>().into()
    }

    /// Synchronously initialize this actor
    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error>
    where
        Self: 'static + Sized + Send + Sync;

    /// Start the actor. This should call `run` if the actor should process events.
    async fn start(&mut self, cx: &mut ActorContext<Self>, data: &mut Self::Data) -> Result<(), Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        self.run(cx, data).await
    }

    /// Run the actor event loop
    async fn run(&mut self, cx: &mut ActorContext<Self>, data: &mut Self::Data) -> Result<(), Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        while let Some(evt) = cx.inbox().next().await {
            // Handle the event
            evt.handle(cx, self, data).await?;
        }
        Ok(())
    }

    /// Handle any processing that needs to happen on shutdown
    async fn shutdown(&mut self, _cx: &mut ActorContext<Self>, _data: &mut Self::Data) -> Result<(), Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        log::debug!("{} shutting down!", self.name());
        Ok(())
    }
}
