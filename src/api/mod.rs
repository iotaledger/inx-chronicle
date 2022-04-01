// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod error;
mod extractors;
mod responses;
mod routes;

use std::{
    task::Poll,
    time::SystemTime,
};

use actix::{
    Actor,
    ActorContext,
    Context,
    Handler,
    Message,
};
use axum::{
    http::{
        Request,
        Response,
    },
    Server,
};
use error::ListenerError;
use futures::Future;
use hyper::{
    Method,
    Uri,
};
use lazy_static::lazy_static;
use mongodb::{
    options::ClientOptions,
    Client,
};
use pin_project::pin_project;
use prometheus::{
    Gauge,
    HistogramOpts,
    HistogramVec,
    IntCounter,
    IntCounterVec,
    Opts,
    Registry,
};
use routes::routes;
use serde::{
    Deserialize,
    Serialize,
};
use tokio::sync::oneshot;
use tower::{
    Layer,
    Service,
};

use crate::config::mongo::MongoConfig;

/// The Chronicle API actor
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ChronicleAPI {
    mongo_config: MongoConfig,
    #[serde(skip)]
    server_handle: Option<oneshot::Sender<()>>,
}

impl ChronicleAPI {
    pub fn new(mongo_config: MongoConfig) -> Self {
        Self {
            mongo_config,
            server_handle: None,
        }
    }
}

impl PartialEq for ChronicleAPI {
    fn eq(&self, other: &Self) -> bool {
        self.mongo_config == other.mongo_config
    }
}

impl Clone for ChronicleAPI {
    fn clone(&self) -> Self {
        Self {
            mongo_config: self.mongo_config.clone(),
            server_handle: None,
        }
    }
}

impl Actor for ChronicleAPI {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        register_metrics();
        let client_opts: ClientOptions = self.mongo_config.clone().into();
        log::info!("Connecting to MongoDB");
        let client = Client::with_options(client_opts)
            .map_err(|e| {
                log::error!("Unable to create client from options, error: {}", e);
            })
            .unwrap();
        let db = client.database("permanode");
        let (sender, receiver) = oneshot::channel();
        log::info!("Starting Axum server");
        tokio::spawn(async {
            Server::bind(&([0, 0, 0, 0], 8000).into())
                .serve(routes(db).into_make_service())
                .with_graceful_shutdown(shutdown_signal(receiver))
                .await
                .unwrap();
        });
        self.server_handle = Some(sender);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let Some(handle) = self.server_handle.take() {
            log::info!("Stopping Axum server");
            handle.send(()).unwrap();
        }
        log::info!("Stopping API");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ShutdownAPI;

impl Handler<ShutdownAPI> for ChronicleAPI {
    type Result = ();

    fn handle(&mut self, _msg: ShutdownAPI, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

async fn shutdown_signal(recv: oneshot::Receiver<()>) {
    if let Err(e) = recv.await {
        log::error!("Error receiving shutdown signal: {}", e);
    }
}

fn register_metrics() {
    REGISTRY
        .register(Box::new(INCOMING_REQUESTS.clone()))
        .expect("Could not register collector");

    REGISTRY
        .register(Box::new(RESPONSE_CODE_COLLECTOR.clone()))
        .expect("Could not register collector");

    REGISTRY
        .register(Box::new(RESPONSE_TIME_COLLECTOR.clone()))
        .expect("Could not register collector");

    REGISTRY
        .register(Box::new(CONFIRMATION_TIME_COLLECTOR.clone()))
        .expect("Could not register collector");
}

lazy_static! {
    /// Metrics registry
    pub static ref REGISTRY: Registry = Registry::new();

    /// Incoming request counter
    pub static ref INCOMING_REQUESTS: IntCounter =
        IntCounter::new("incoming_requests", "Incoming Requests").expect("failed to create metric");

    /// Response code collector
    pub static ref RESPONSE_CODE_COLLECTOR: IntCounterVec = IntCounterVec::new(
        Opts::new("response_code", "Response Codes"),
        &["statuscode", "type"]
    )
    .expect("failed to create metric");

    /// Response time collector
    pub static ref RESPONSE_TIME_COLLECTOR: HistogramVec =
        HistogramVec::new(HistogramOpts::new("response_time", "Response Times"), &["endpoint"])
            .expect("failed to create metric");

    /// Confirmation time collector
    pub static ref CONFIRMATION_TIME_COLLECTOR: Gauge =
        Gauge::new("confirmation_time", "Confirmation Times")
            .expect("failed to create metric");
}

#[derive(Clone, Debug)]
pub struct Metrics<T> {
    inner: T,
}

impl<T> Metrics<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<S, R, Res> Service<Request<R>> for Metrics<S>
where
    S: Service<Request<R>, Response = Response<Res>>,
    S::Error: 'static,
    S::Future: 'static,
    S::Response: 'static,
    Res: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = MetricsResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request<R>) -> Self::Future {
        let start_time = SystemTime::now();
        INCOMING_REQUESTS.inc();
        let method = req.method().clone();
        let uri = req.uri().clone();
        MetricsResponseFuture {
            fut: self.inner.call(req),
            start_time,
            method,
            uri,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer {
    type Service = Metrics<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Metrics::new(inner)
    }
}

#[pin_project]
pub struct MetricsResponseFuture<F> {
    #[pin]
    fut: F,
    #[pin]
    start_time: SystemTime,
    #[pin]
    method: Method,
    #[pin]
    uri: Uri,
}

impl<F, Res, Err> Future for MetricsResponseFuture<F>
where
    F: Future<Output = Result<Response<Res>, Err>>,
{
    type Output = Result<Response<Res>, Err>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.fut.poll(cx) {
            Poll::Ready(res) => {
                let duration = this.start_time.elapsed().unwrap();
                let ms = (duration.as_secs() * 1000 + duration.subsec_millis() as u64) as f64;
                RESPONSE_TIME_COLLECTOR
                    .with_label_values(&[&format!("{} {}", this.method, this.uri)])
                    .observe(ms);
                if let Ok(res) = res.as_ref() {
                    match res.status().as_u16() {
                        500..=599 => RESPONSE_CODE_COLLECTOR
                            .with_label_values(&[res.status().as_str(), "500"])
                            .inc(),
                        400..=499 => RESPONSE_CODE_COLLECTOR
                            .with_label_values(&[res.status().as_str(), "400"])
                            .inc(),
                        300..=399 => RESPONSE_CODE_COLLECTOR
                            .with_label_values(&[res.status().as_str(), "300"])
                            .inc(),
                        200..=299 => RESPONSE_CODE_COLLECTOR
                            .with_label_values(&[res.status().as_str(), "200"])
                            .inc(),
                        100..=199 => RESPONSE_CODE_COLLECTOR
                            .with_label_values(&[res.status().as_str(), "100"])
                            .inc(),
                        _ => (),
                    }
                }
                Poll::Ready(res)
            }
            p => p,
        }
    }
}
