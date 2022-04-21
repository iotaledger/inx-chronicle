// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{pin::Pin, task::Poll, time::SystemTime};

use axum::{routing::get, Router};
use futures::Future;
use hyper::{Method, Request, Response, Uri};
use prometheus::{Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, TextEncoder};
use tower::{Layer, Service};

use super::error::ApiError;
use crate::REGISTRY;

#[derive(Clone, Debug)]
pub struct ApiMetrics {
    /// Incoming request counter
    pub incoming_requests: IntCounter,
    /// Response code collector
    pub response_code_counter: IntCounterVec,
    /// Response time collector
    pub response_time_collector: HistogramVec,
}

impl ApiMetrics {
    pub fn new() -> Self {
        // Panic: If any of this fails, there's not much we can do. The app should close, so panicking is ok.
        let res = Self {
            incoming_requests: IntCounter::new("incoming_requests", "Incoming Requests").unwrap(),
            response_code_counter: IntCounterVec::new(
                Opts::new("response_code", "Response Codes"),
                &["statuscode", "type"],
            )
            .unwrap(),
            response_time_collector: HistogramVec::new(
                HistogramOpts::new("response_time", "Response Times"),
                &["endpoint"],
            )
            .unwrap(),
        };
        REGISTRY.register(Box::new(res.incoming_requests.clone())).unwrap();
        REGISTRY.register(Box::new(res.response_code_counter.clone())).unwrap();
        REGISTRY
            .register(Box::new(res.response_time_collector.clone()))
            .unwrap();
        res
    }
}

pub fn routes() -> Router {
    Router::new().route("/metrics", get(metrics))
}

async fn metrics() -> Result<String, ApiError> {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&REGISTRY.gather(), &mut buffer)?;

    encoder.encode(&prometheus::gather(), &mut buffer)?;

    Ok(String::from_utf8(buffer)?)
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
    S::Future: 'static + Unpin,
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
        // Unwrap: This extension should always exist if compiled with the `metrics` feature.
        let metrics = req.extensions().get::<ApiMetrics>().unwrap().clone();
        metrics.incoming_requests.inc();
        let method = req.method().clone();
        let uri = req.uri().clone();
        MetricsResponseFuture {
            fut: self.inner.call(req),
            metrics,
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

pub struct MetricsResponseFuture<F: Unpin> {
    fut: F,
    metrics: ApiMetrics,
    start_time: SystemTime,
    method: Method,
    uri: Uri,
}

impl<F, Res, Err> Future for MetricsResponseFuture<F>
where
    F: Future<Output = Result<Response<Res>, Err>> + Unpin,
{
    type Output = Result<Response<Res>, Err>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let fut = Pin::new(&mut this.fut);
        match fut.poll(cx) {
            Poll::Ready(res) => {
                // Unwrap: Shouldn't be an issue with time-traveling, but if there is, we should gracefully catch this
                // with the panic layer.
                let duration = this.start_time.elapsed().unwrap();
                let ms = (duration.as_secs() * 1000 + duration.subsec_millis() as u64) as f64;
                this.metrics
                    .response_time_collector
                    .with_label_values(&[&format!("{} {}", this.method, this.uri)])
                    .observe(ms);
                if let Ok(res) = res.as_ref() {
                    let status_bucket = ((res.status().as_u16() / 100) * 100).to_string();
                    this.metrics
                        .response_code_counter
                        .with_label_values(&[res.status().as_str(), &status_bucket])
                        .inc()
                }
                Poll::Ready(res)
            }
            p => p,
        }
    }
}
