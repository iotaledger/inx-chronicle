// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
};

use async_trait::async_trait;
use bee_metrics::{metrics::process::ProcessMetrics, serve_metrics};
use chronicle::runtime::{Actor, ActorContext, Task};
use futures::future::AbortHandle;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{sleep, Duration},
};

pub struct MetricsWorker {
    config: MetricsConfig,
}

impl MetricsWorker {
    pub fn new(config: MetricsConfig) -> Self {
        Self { config }
    }
}

pub struct MetricsState {
    server_handle: (JoinHandle<()>, Option<oneshot::Sender<()>>),
    abort_handles: Vec<AbortHandle>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricsConfig {
    address: IpAddr,
    port: u16,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            address: [0, 0, 0, 0].into(),
            port: 9100,
        }
    }
}

struct UpdateProcessMetrics {
    process_metrics: ProcessMetrics,
}

#[async_trait]
impl Task for UpdateProcessMetrics {
    type Error = std::convert::Infallible;

    async fn run(self) -> Result<(), Self::Error> {
        const MAX_RETRIES: u8 = 5;
        let mut retries = MAX_RETRIES;

        while retries > 0 {
            sleep(Duration::from_secs(1)).await;

            match self.process_metrics.update().await {
                Ok(_) => {
                    retries = MAX_RETRIES;
                }
                Err(e) => {
                    log::warn!("Cannot update process metrics: {e}");
                    retries -= 1;
                }
            }
        }

        log::warn!("Could not update process metrics after {MAX_RETRIES} retries");

        Ok(())
    }
}

#[async_trait]
impl Actor for MetricsWorker {
    type State = MetricsState;
    type Error = Infallible;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let addr = SocketAddr::new(self.config.address, self.config.port);

        let process_metrics = ProcessMetrics::new(std::process::id());
        let (mem_metric, cpu_metric) = process_metrics.metrics();

        cx.metrics_registry()
            .register("memory_usage", "Memory usage", mem_metric);
        cx.metrics_registry().register("cpu_usage", "CPU usage", cpu_metric);

        let (send, recv) = oneshot::channel();
        let metrics_handle = cx.handle().clone();

        let server_fut = {
            let registry = cx.metrics_registry().clone();
            tokio::spawn(async move {
                let fut = serve_metrics(addr, registry);

                // FIXME: use `with_graceful_shutdown` when `bee-metrics` exposes the complete
                // future type.
                let res = tokio::select! {
                    res = fut => { res }
                    _ = recv => { Ok(()) }
                };

                // Stop the actor if the server stops.
                metrics_handle.shutdown();

                res.unwrap()
            })
        };

        let abort_handles = vec![cx.spawn_task(UpdateProcessMetrics { process_metrics }).await];

        Ok(MetricsState {
            server_handle: (server_fut, Some(send)),
            abort_handles,
        })
    }

    async fn shutdown(
        &mut self,
        cx: &mut ActorContext<Self>,
        state: &mut Self::State,
        run_result: Result<(), Self::Error>,
    ) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());

        state.server_handle.1.take().map(|send| send.send(()));

        for abort_handle in &mut state.abort_handles {
            abort_handle.abort();
        }

        run_result
    }
}
