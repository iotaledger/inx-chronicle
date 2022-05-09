// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{convert::Infallible, sync::Arc};

use async_trait::async_trait;
use bee_metrics::{encoding::SendSyncEncodeMetric, metrics::ProcessMetrics, serve_metrics, Registry};
use chronicle::runtime::{Actor, ActorContext, HandleEvent};
use futures::future::FutureExt;
use tokio::{
    task::JoinHandle,
    time::{sleep, Duration},
};

#[derive(Default)]
pub struct MetricsWorker {}

#[async_trait]
impl Actor for MetricsWorker {
    type State = (Arc<Registry>, JoinHandle<()>, JoinHandle<()>);

    type Error = Infallible;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let registry = Arc::new(Registry::default());

        // FIXME: pass address via config.
        let addr = "0.0.0.0:6969".parse().unwrap();

        let server_fut = tokio::spawn(serve_metrics(addr, registry.clone()).map(|res| res.unwrap()));

        let metrics = ProcessMetrics::new(std::process::id());
        let (mem_metric, cpu_metric) = metrics.metrics();

        registry.register("memory_usage", "Memory usage", mem_metric);
        registry.register("cpu_usage", "CPU usage", cpu_metric);

        let process_metrics_fut = {
            tokio::spawn(async move {
                loop {
                    metrics.update().await;
                    sleep(Duration::from_secs(1)).await;
                }
            })
        };

        Ok((registry, server_fut, process_metrics_fut))
    }

    async fn shutdown(
        &mut self,
        cx: &mut ActorContext<Self>,
        (_, server_fut, process_metrics_fut): &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());

        process_metrics_fut.abort();

        // FIXME: is this good enough to stop the task?
        server_fut.abort();

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
        (registry, _, _): &mut Self::State,
    ) -> Result<(), Self::Error> {
        registry.register(event.name, event.help, event.metric);

        Ok(())
    }
}
