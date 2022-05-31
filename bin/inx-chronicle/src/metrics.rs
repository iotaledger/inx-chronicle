// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
};

use async_trait::async_trait;
use bee_metrics::{
    metrics::{gauge::Gauge, process::ProcessMetrics},
    serve_metrics,
};
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent, Task, TaskError, TaskReport},
};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{sleep, Duration},
};

pub struct MetricsWorker {
    db: MongoDb,
    config: MetricsConfig,
}

impl MetricsWorker {
    pub fn new(db: MongoDb, config: MetricsConfig) -> Self {
        Self { db, config }
    }
}

pub struct MetricsState {
    server_handle: (JoinHandle<()>, Option<oneshot::Sender<()>>),
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

    async fn run(&mut self) -> Result<(), Self::Error> {
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

struct UpdateDbMetrics {
    db: MongoDb,
    size: Gauge<u64>,
}

#[async_trait]
impl Task for UpdateDbMetrics {
    type Error = std::convert::Infallible;

    async fn run(&mut self) -> Result<(), Self::Error> {
        const MAX_RETRIES: u8 = 5;
        let mut retries = MAX_RETRIES;

        while retries > 0 {
            sleep(Duration::from_secs(60)).await;

            match self.db.size().await {
                Ok(size) => {
                    self.size.set(size);
                    retries = MAX_RETRIES;
                }
                Err(e) => {
                    log::warn!("Cannot update database metrics: {e}");
                    retries -= 1;
                }
            }
        }

        log::warn!("Could not update databse metrics after {MAX_RETRIES} retries");

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

        let db_size_metric = Gauge::<u64>::default();

        cx.metrics_registry()
            .register("memory_usage", "Memory usage", mem_metric);
        cx.metrics_registry().register("cpu_usage", "CPU usage", cpu_metric);

        cx.metrics_registry()
            .register("db_size", "DB size", db_size_metric.clone());

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

        cx.spawn_child_task(UpdateProcessMetrics { process_metrics }).await;
        cx.spawn_child_task(UpdateDbMetrics {
            db: self.db.clone(),
            size: db_size_metric,
        })
        .await;

        Ok(MetricsState {
            server_handle: (server_fut, Some(send)),
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

        run_result
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Metrics Worker".into()
    }
}

#[async_trait]
impl HandleEvent<TaskReport<UpdateProcessMetrics>> for MetricsWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: TaskReport<UpdateProcessMetrics>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            TaskReport::Success(_) => {}
            TaskReport::Error(report) => match report.error {
                TaskError::Aborted => {}
                TaskError::Panic => {
                    cx.spawn_child_task(report.take_task()).await;
                }
                TaskError::Result(err) => match err {},
            },
        }

        Ok(())
    }
}

#[async_trait]
impl HandleEvent<TaskReport<UpdateDbMetrics>> for MetricsWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: TaskReport<UpdateDbMetrics>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            TaskReport::Success(_) => {}
            TaskReport::Error(report) => match report.error {
                TaskError::Aborted => {}
                TaskError::Panic => {
                    cx.spawn_child_task(report.take_task()).await;
                }
                TaskError::Result(err) => match err {},
            },
        }

        Ok(())
    }
}
