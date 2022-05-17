// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::task::{Context, Poll};

use bee_metrics::metrics::counter::Counter;
use hyper::{Request, Response};
use tower::{Layer, Service};

#[derive(Default, Clone)]
pub(super) struct Metrics {
    pub(super) incoming_requests: Counter,
}

#[derive(Clone)]
pub(super) struct MetricsService<T> {
    inner: T,
    metrics: Metrics,
}

impl<S, R, Res> Service<Request<R>> for MetricsService<S>
where
    S: Service<Request<R>, Response = Response<Res>>,
    S::Error: 'static,
    S::Future: 'static + Unpin,
    S::Response: 'static,
    Res: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request<R>) -> Self::Future {
        self.metrics.incoming_requests.inc();

        self.inner.call(req)
    }
}

#[derive(Default)]
pub(super) struct MetricsLayer {
    pub(super) metrics: Metrics,
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            metrics: self.metrics.clone(),
        }
    }
}
