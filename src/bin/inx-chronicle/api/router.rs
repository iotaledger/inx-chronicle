// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! This `Router` wraps the functionality we use from [`axum::Router`] and tracks the string routes
//! as they are added in a tree node structure. The reason for this ugliness is to provide a routes
//! endpoint which can output a list of unique routes at any depth level. The most critical part of
//! this is the [`Router::into_make_service()`] function, which adds an [`Extension`] containing the
//! root [`RouteNode`]. These routes can also be filtered using a [`RegexSet`] to allow the exclusion
//! of unauthorized routes.

use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    convert::Infallible,
};

use axum::{
    body::HttpBody,
    handler::Handler,
    response::{IntoResponse, Response},
    routing::{future::RouteFuture, MethodRouter, Route},
};
use hyper::{Body, Request};
use regex::RegexSet;
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

    pub fn list_routes(&self, public_routes: impl Into<Option<RegexSet>>, depth: Option<usize>) -> Vec<String> {
        let mut routes = BTreeSet::new();
        self.list_routes_recursive(&mut Vec::new(), &mut routes, &public_routes.into(), depth);
        routes.into_iter().collect()
    }

    fn list_routes_recursive(
        &self,
        parents: &mut Vec<String>,
        routes: &mut BTreeSet<String>,
        public_routes: &Option<RegexSet>,
        depth: Option<usize>,
    ) {
        if self.children.is_empty() {
            let mut route = parents.join("");
            let pieces = route.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>();
            if public_routes.is_none() || matches!(public_routes, Some(public_routes) if public_routes.is_match(&route))
            {
                if let Some(depth) = depth {
                    if depth < pieces.len() {
                        route = pieces[..depth].join("/");
                    } else {
                        route = pieces.join("/");
                    }
                }
                routes.insert(route);
            }
        }
        for (name, child) in self.children.iter() {
            parents.push(name.clone());
            child.list_routes_recursive(parents, routes, public_routes, depth);
            parents.pop();
        }
    }
}

#[derive(Debug)]
pub struct Router<S = (), B = Body> {
    inner: axum::Router<S, B>,
    root: RouteNode,
}

impl<S, B> Clone for Router<S, B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            root: self.root.clone(),
        }
    }
}

impl<S, B> Default for Router<S, B>
where
    Router<S, B>: Default,
{
    fn default() -> Self {
        Self::default()
    }
}

impl<S, B> Router<S, B>
where
    B: HttpBody + Send + 'static,
    S: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            inner: axum::Router::new(),
            root: Default::default(),
        }
    }

    pub fn route(mut self, path: &str, method_router: MethodRouter<S, B>) -> Self {
        self.root.children.entry(path.to_string()).or_default();
        Self {
            inner: self.inner.route(path, method_router),
            root: self.root,
        }
    }

    pub fn nest(mut self, path: &str, router: Router<S, B>) -> Self {
        match self.root.children.entry(path.to_string()) {
            Entry::Occupied(mut o) => o.get_mut().merge(router.root),
            Entry::Vacant(v) => {
                v.insert(router.root);
            }
        }
        Self {
            inner: self.inner.nest(path, router.inner),
            root: self.root,
        }
    }

    pub fn layer<L, NewReqBody>(self, layer: L) -> Router<S, NewReqBody>
    where
        L: Layer<Route<B>> + Clone + Send + 'static,
        L::Service: Service<Request<NewReqBody>> + Clone + Send + 'static,
        <L::Service as Service<Request<NewReqBody>>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request<NewReqBody>>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request<NewReqBody>>>::Future: Send + 'static,
        NewReqBody: HttpBody + 'static,
    {
        Router {
            inner: self.inner.layer(layer),
            root: self.root,
        }
    }

    pub fn route_layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route<B>> + Clone + Send + 'static,
        L::Service: Service<Request<B>> + Clone + Send + 'static,
        <L::Service as Service<Request<B>>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request<B>>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request<B>>>::Future: Send + 'static,
    {
        Self {
            inner: self.inner.route_layer(layer),
            root: self.root,
        }
    }

    pub fn fallback<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S, B>,
        T: 'static,
    {
        Self {
            inner: self.inner.fallback(handler),
            root: self.root,
        }
    }

    pub fn finish(self) -> (RouteNode, axum::Router<S, B>) {
        (self.root, self.inner)
    }
}

impl<B> Service<Request<B>> for Router<(), B>
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
