// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{task::Poll, time::SystemTime};

use axum::{routing::get, Router};
use futures::Future;
use hyper::{Method, Request, Response, Uri};
use lazy_static::lazy_static;
use pin_project::pin_project;
use prometheus::{Encoder, Gauge, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, Registry, TextEncoder};
use tower::{Layer, Service};

use super::error::ListenerError;

pub fn routes() -> Router {
    Router::new().route("/metrics", get(metrics))
}

async fn metrics() -> Result<String, ListenerError> {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder
        .encode(&REGISTRY.gather(), &mut buffer)
        .map_err(ListenerError::other)?;

    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .map_err(ListenerError::other)?;

    String::from_utf8(buffer).map_err(ListenerError::other)
}

pub fn register_metrics() {
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
                    let status_bucket = ((res.status().as_u16() / 100) * 100).to_string();
                    RESPONSE_CODE_COLLECTOR
                        .with_label_values(&[res.status().as_str(), &status_bucket])
                        .inc()
                }
                Poll::Ready(res)
            }
            p => p,
        }
    }
}
