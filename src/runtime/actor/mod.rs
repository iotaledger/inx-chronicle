// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing the actor address handle.
pub(crate) mod addr;
/// Module containing the actor context.
pub(crate) mod context;
/// Module containing actor error types.
pub(crate) mod error;
/// Module containing event types.
pub(crate) mod event;
/// Module containing actor exit report types.
pub(crate) mod report;
/// Module containing event sender traits.
pub(crate) mod sender;
/// Module containing utilities.
pub(crate) mod util;

use std::{borrow::Cow, error::Error};

use async_trait::async_trait;
use futures::StreamExt;

use self::context::ActorContext;

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
        #[cfg(feature = "metrics")]
        let histogram = {
            let histogram = bee_metrics::metrics::histogram::Histogram::new(
                bee_metrics::metrics::histogram::exponential_buckets(1.0, 2.0, 10),
            );
            cx.metrics_registry().register(
                format!("{}_loop_time", util::sanitize_metric_name(self.name().as_ref())),
                format!("{} loop timings", self.name()),
                histogram.clone(),
            );
            histogram
        };
        while let Some(evt) = cx.inbox().next().await {
            #[cfg(feature = "metrics")]
            let start_time = std::time::Instant::now();
            // Handle the event
            evt.handle(cx, self, state).await?;
            #[cfg(feature = "metrics")]
            {
                let elapsed = start_time.elapsed();
                histogram.observe(elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0);
            }
        }

        log::debug!("{} exited event loop ({})", self.name(), cx.id());
        Ok(())
    }

    /// Handle any processing that needs to happen on shutdown
    async fn shutdown(
        &mut self,
        cx: &mut ActorContext<Self>,
        _state: &mut Self::State,
        run_result: Result<(), Self::Error>,
    ) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());
        run_result
    }
}
