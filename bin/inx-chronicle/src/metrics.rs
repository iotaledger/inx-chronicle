// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
};

use async_trait::async_trait;
// use bee_metrics::{metrics::gauge::Gauge, serve_metrics};
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent, Task, TaskError, TaskReport},
};
// use futures::StreamExt;
use metrics::{describe_gauge, describe_histogram, gauge, Unit};
use metrics_exporter_prometheus::PrometheusBuilder;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

const GAPS_COUNT: &str = "db_gaps_count";
const GAPS_UPDATE_INTERVAL_DEFAULT: Duration = Duration::from_secs(60);
pub(crate) const SYNC_TIME: &str = "ms_sync_time";

pub struct MetricsWorker {
    db: MongoDb,
    config: MetricsConfig,
}

impl MetricsWorker {
    pub fn new(db: &MongoDb, config: &MetricsConfig) -> Self {
        Self {
            db: db.clone(),
            config: config.clone(),
        }
    }
}

pub struct MetricsState {
    // server_handle: (JoinHandle<()>, Option<oneshot::Sender<()>>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub address: IpAddr,
    pub port: u16,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            address: [0, 0, 0, 0].into(),
            port: 9100,
        }
    }
}
struct GapsMetric {
    db: MongoDb,
}

#[async_trait]
impl Task for GapsMetric {
    type Error = std::convert::Infallible;

    async fn run(&mut self) -> Result<(), Self::Error> {
        const MAX_RETRIES: u8 = 5;
        let mut remaining_retries = MAX_RETRIES;

        while remaining_retries > 0 {
            sleep(GAPS_UPDATE_INTERVAL_DEFAULT).await;

            // TODO: we need a gaps collection to actually make this efficient.
            match self.db.get_gaps().await {
                Ok(gaps) => {
                    let num_gaps = gaps
                        .iter()
                        .map(|range| ((**range.end() + 1) - **range.start()))
                        .sum::<u32>() as f64;

                    gauge!(GAPS_COUNT, num_gaps);

                    remaining_retries = MAX_RETRIES;
                }
                Err(e) => {
                    log::warn!("Cannot update database metrics: {e}");
                    remaining_retries -= 1;
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

        // TODO: get the server handle!
        let builder = PrometheusBuilder::new();
        builder
            .with_http_listener(addr)
            .install()
            // TODO: return actual error
            .expect("failed to install Prometheus recorder");

        describe_gauge!(GAPS_COUNT, Unit::Count, "the current number of gaps in the database");
        describe_histogram!(SYNC_TIME, Unit::Milliseconds, "the time it took to sync a milestone");

        // let db_size_metric = Gauge::<u64>::default();

        // cx.metrics_registry()
        //     .register("db_size", "DB size", db_size_metric.clone());

        // let (send, recv) = oneshot::channel();
        // let metrics_handle = cx.handle().clone();

        // let server_fut = {
        //     let registry = cx.metrics_registry().clone();
        //     tokio::spawn(async move {
        //         let res = serve_metrics(addr, registry)
        //             .with_graceful_shutdown(async {
        //                 recv.await.ok();
        //             })
        //             .await;

        //         // Stop the actor if the server stops.
        //         metrics_handle.shutdown().await;

        //         res.unwrap()
        //     })
        // };

        cx.spawn_child_task(GapsMetric { db: self.db.clone() }).await;

        Ok(MetricsState {
            // server_handle: (server_fut, Some(send)),
        })
    }

    async fn shutdown(
        &mut self,
        cx: &mut ActorContext<Self>,
        _state: &mut Self::State,
        run_result: Result<(), Self::Error>,
    ) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());

        // state.server_handle.1.take().map(|send| send.send(()));

        run_result
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Metrics Worker".into()
    }
}

#[async_trait]
impl HandleEvent<TaskReport<GapsMetric>> for MetricsWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: TaskReport<GapsMetric>,
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
