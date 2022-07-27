// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, SocketAddr};

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ErrorLevel, HandleEvent, Task, TaskError, TaskReport},
};
use metrics::{describe_gauge, describe_histogram, gauge, Unit};
use metrics_exporter_prometheus::{BuildError as PrometheusBuildError, PrometheusBuilder};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::oneshot,
    time::{sleep, Duration},
};

const GAPS_COUNT: &str = "db_gaps_count";
const GAPS_UPDATE_INTERVAL_DEFAULT: Duration = Duration::from_secs(60);
pub(crate) const SYNC_TIME: &str = "ms_sync_time";

#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error(transparent)]
    Prometheus(#[from] PrometheusBuildError),
}

impl ErrorLevel for MetricsError {
    fn level(&self) -> log::Level {
        match self {
            Self::Prometheus(_) => log::Level::Warn,
        }
    }
}

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
    shutdown_tx: Option<oneshot::Sender<()>>,
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

        log::warn!("Could not update gaps metric after {MAX_RETRIES} retries");

        Ok(())
    }
}

#[async_trait]
impl Actor for MetricsWorker {
    type State = MetricsState;
    type Error = MetricsError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let addr = SocketAddr::new(self.config.address, self.config.port);

        let (recorder, exporter_fut) = PrometheusBuilder::new().with_http_listener(addr).build()?;

        metrics::set_boxed_recorder(Box::new(recorder)).map_err(PrometheusBuildError::from)?;

        describe_gauge!(GAPS_COUNT, Unit::Count, "the current number of gaps in the database");
        describe_histogram!(SYNC_TIME, Unit::Seconds, "the time it took to sync a milestone");

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let worker_handle = cx.handle().clone();

        tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_rx => {}
                _ = exporter_fut => {}
            }

            // Stop the actor if the server stops.
            worker_handle.shutdown().await;
        });

        cx.spawn_child_task(GapsMetric { db: self.db.clone() }).await;

        Ok(MetricsState {
            shutdown_tx: Some(shutdown_tx),
        })
    }

    async fn shutdown(
        &mut self,
        cx: &mut ActorContext<Self>,
        state: &mut Self::State,
        run_result: Result<(), Self::Error>,
    ) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());

        state.shutdown_tx.take().map(|tx| tx.send(()));

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
