// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{convert::Infallible, sync::Arc};

use async_trait::async_trait;
use bee_metrics::{encoding::SendSyncEncodeMetric, serve_metrics, Registry};
use chronicle::runtime::{Actor, ActorContext, HandleEvent};
use futures::future::FutureExt;
use tokio::task::JoinHandle;

#[derive(Default)]
pub struct MetricsWorker {}

#[async_trait]
impl Actor for MetricsWorker {
    type State = (Arc<Registry>, JoinHandle<()>);

    type Error = Infallible;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let registry = Arc::new(Registry::default());

        // FIXME: pass address via config.
        let addr = "0.0.0.0:6969".parse().unwrap();
        let fut = tokio::spawn(serve_metrics(addr, registry.clone()).map(|res| res.unwrap()));

        Ok((registry, fut))
    }

    async fn shutdown(&mut self, cx: &mut ActorContext<Self>, (_, fut): &mut Self::State) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());

        // FIXME: is this good enough to stop the task?
        fut.abort();

        Ok(())
    }
}

pub struct RegisterMetric<M: 'static + SendSyncEncodeMetric> {
    pub name: String,
    pub help: String,
    pub metric: M,
}

#[async_trait]
impl<M: 'static + SendSyncEncodeMetric> HandleEvent<RegisterMetric<M>> for MetricsWorker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        event: RegisterMetric<M>,
        (registry, _): &mut Self::State,
    ) -> Result<(), Self::Error> {
        registry.register(event.name, event.help, event.metric);

        Ok(())
    }
}
