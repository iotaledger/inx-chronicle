// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! This `Router` wraps the functionality we use from [`axum::Router`] and tracks the string routes
//! as they are added in a tree node structure. The reason for this ugliness is to provide a routes
//! endpoint which can output a list of unique routes at any depth level. The most critical part of
//! this is the [`Router::into_make_service()`] function, which adds an [`Extension`] containing the
//! root [`RouteNode`]. These routes can also be filtered using a [`RegexSet`] to allow the exclusion
//! of unauthorized routes.

use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
};

use axum::{
    body::HttpBody,
    extract::FromRef,
    handler::Handler,
    response::IntoResponse,
    routing::{MethodRouter, Route},
};
use chronicle::db::MongoDb;
use hyper::{Body, Request};
use regex::RegexSet;
use tower::{Layer, Service};

use super::{ApiData, ApiWorker};

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

#[derive(Debug, Clone)]
pub struct RouterState<S> {
    pub inner: S,
    pub routes: RouteNode,
}

impl FromRef<RouterState<ApiWorker>> for MongoDb {
    fn from_ref(input: &RouterState<ApiWorker>) -> Self {
        input.inner.db.clone()
    }
}

impl FromRef<RouterState<ApiWorker>> for ApiData {
    fn from_ref(input: &RouterState<ApiWorker>) -> Self {
        input.inner.api_data.clone()
    }
}

impl FromRef<RouterState<ApiWorker>> for ApiWorker {
    fn from_ref(input: &RouterState<ApiWorker>) -> Self {
        input.inner.clone()
    }
}

impl<S> FromRef<RouterState<S>> for RouteNode {
    fn from_ref(input: &RouterState<S>) -> Self {
        input.routes.clone()
    }
}

#[derive(Debug)]
pub struct Router<S = (), B = Body> {
    inner: axum::Router<RouterState<S>, B>,
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
    B: HttpBody + Send + 'static,
    S: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
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

    pub fn route(mut self, path: &str, method_router: MethodRouter<RouterState<S>, B>) -> Self {
        self.root.children.entry(path.to_string()).or_default();
        Self {
            inner: self.inner.route(path, method_router),
            root: self.root,
        }
    }

    pub fn route_service<T>(self, path: &str, service: T) -> Self
    where
        T: Service<Request<B>, Error = Infallible> + Clone + Send + 'static,
        T::Response: IntoResponse,
        T::Future: Send + 'static,
    {
        Self {
            inner: self.inner.route_service(path, service),
            root: self.root,
        }
    }

    pub fn nest(mut self, path: &str, router: Router<S, B>) -> Self {
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

    pub fn merge<R>(mut self, other: R) -> Self
    where
        R: Into<Router<S, B>>,
    {
        let other = other.into();
        self.root.merge(other.root);
        Self {
            inner: self.inner.merge(other.inner),
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
        H: Handler<T, RouterState<S>, B>,
        T: 'static,
    {
        Self {
            inner: self.inner.fallback(handler),
            root: self.root,
        }
    }

    pub fn with_state<S2>(self, state: S) -> axum::Router<S2, B> {
        self.inner.with_state(RouterState {
            inner: state,
            routes: self.root,
        })
    }
}
