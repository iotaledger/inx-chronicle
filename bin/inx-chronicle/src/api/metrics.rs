// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::task::{Context, Poll};

use hyper::{Request, Response};
use metrics::increment_counter;
use tower::{Layer, Service};

use crate::metrics::REQ_COUNT;

#[derive(Clone)]
pub(super) struct MetricsService<T> {
    inner: T,
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
        increment_counter!(REQ_COUNT);
        self.inner.call(req)
    }
}

#[derive(Default)]
pub(super) struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service { inner }
    }
}
