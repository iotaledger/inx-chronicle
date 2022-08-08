// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, convert::Infallible};

use axum::{
    body::{Bytes, HttpBody},
    response::Response,
    routing::{future::RouteFuture, IntoMakeService, Route},
    BoxError, Extension,
};
use hyper::{Body, Request};
use tower::{Layer, Service};

#[derive(Clone, Debug, Default)]
pub struct RouteNode {
    children: BTreeMap<String, RouteNode>,
}

impl RouteNode {
    fn merge(&mut self, other: RouteNode) {
        use std::collections::btree_map::Entry::*;
        for (name, child) in other.children {
            match self.children.entry(name) {
                Occupied(mut o) => o.get_mut().merge(child),
                Vacant(v) => {
                    v.insert(child);
                }
            }
        }
    }

    pub fn list_routes(&self) -> Vec<String> {
        let mut routes = Vec::new();
        self.list_routes_recursive(&mut Vec::new(), &mut routes);
        routes
    }

    fn list_routes_recursive(&self, parents: &mut Vec<String>, routes: &mut Vec<String>) {
        if self.children.is_empty() {
            let mut route = parents.join("");
            while route.ends_with("/") {
                route.pop();
            }
            routes.push(route);
        }
        for (name, child) in self.children.iter() {
            parents.push(name.clone());
            child.list_routes_recursive(parents, routes);
            parents.pop();
        }
    }
}

#[derive(Debug)]
pub struct Router<B = Body> {
    inner: axum::Router<B>,
    root: RouteNode,
}

impl<B> Clone for Router<B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            root: self.root.clone(),
        }
    }
}

impl<B> Default for Router<B>
where
    B: HttpBody + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<B> Router<B>
where
    B: HttpBody + Send + 'static,
{
    pub fn new() -> Self {
        Self {
            inner: axum::Router::new(),
            root: Default::default(),
        }
    }

    pub fn route<T>(mut self, path: &str, service: T) -> Self
    where
        T: Service<Request<B>, Response = Response, Error = Infallible> + Clone + Send + 'static,
        T::Future: Send + 'static,
    {
        self.root.children.entry(path.to_string()).or_default();
        Self {
            inner: self.inner.route(path, service),
            root: self.root,
        }
    }

    pub fn nest<T>(mut self, path: &str, service: T) -> Self
    where
        T: Service<Request<B>, Response = Response, Error = Infallible> + Clone + Send + 'static,
        T::Future: Send + 'static,
    {
        match try_downcast::<Router<B>, _>(service) {
            Ok(router) => {
                match self.root.children.entry(path.to_string()) {
                    std::collections::btree_map::Entry::Occupied(mut o) => o.get_mut().merge(router.root),
                    std::collections::btree_map::Entry::Vacant(v) => {
                        v.insert(router.root);
                    }
                }
                Self {
                    inner: self.inner.nest(path, router.inner),
                    root: self.root,
                }
            }
            Err(service) => Self {
                inner: self.inner.nest(path, service),
                root: self.root,
            },
        }
    }

    pub fn merge<R>(mut self, other: R) -> Self
    where
        R: Into<Router<B>>,
    {
        let other = other.into();
        self.root.merge(other.root);
        Self {
            inner: self.inner.merge(other.inner),
            root: self.root,
        }
    }

    pub fn layer<L, NewReqBody, NewResBody>(self, layer: L) -> Router<NewReqBody>
    where
        L: Layer<Route<B>>,
        L::Service:
            Service<Request<NewReqBody>, Response = Response<NewResBody>, Error = Infallible> + Clone + Send + 'static,
        <L::Service as Service<Request<NewReqBody>>>::Future: Send + 'static,
        NewResBody: HttpBody<Data = Bytes> + Send + 'static,
        NewResBody::Error: Into<BoxError>,
    {
        Router {
            inner: self.inner.layer(layer),
            root: self.root,
        }
    }

    pub fn route_layer<L, NewResBody>(self, layer: L) -> Self
    where
        L: Layer<Route<B>>,
        L::Service: Service<Request<B>, Response = Response<NewResBody>, Error = Infallible> + Clone + Send + 'static,
        <L::Service as Service<Request<B>>>::Future: Send + 'static,
        NewResBody: HttpBody<Data = Bytes> + Send + 'static,
        NewResBody::Error: Into<BoxError>,
    {
        Self {
            inner: self.inner.route_layer(layer),
            root: self.root,
        }
    }

    pub fn fallback<T>(self, service: T) -> Self
    where
        T: Service<Request<B>, Response = Response, Error = Infallible> + Clone + Send + 'static,
        T::Future: Send + 'static,
    {
        Self {
            inner: self.inner.fallback(service),
            root: self.root,
        }
    }

    pub fn into_make_service(self) -> IntoMakeService<axum::Router<B>> {
        self.inner.layer(Extension(self.root)).into_make_service()
    }
}

impl<B> Service<Request<B>> for Router<B>
where
    B: HttpBody + Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = RouteFuture<B, Infallible>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        self.inner.call(req)
    }
}

fn try_downcast<T, K>(k: K) -> Result<T, K>
where
    T: 'static,
    K: Send + 'static,
{
    let mut k = Some(k);
    if let Some(k) = <dyn std::any::Any>::downcast_mut::<Option<T>>(&mut k) {
        Ok(k.take().unwrap())
    } else {
        Err(k.unwrap())
    }
}
