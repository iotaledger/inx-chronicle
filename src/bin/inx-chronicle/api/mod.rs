// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Contains routes that can be used to access data stored by Chronicle
//! as well as the health of the application and analytics.

mod extractors;

#[cfg(feature = "stardust")]
pub mod stardust;

mod error;
mod secret_key;
#[macro_use]
mod responses;
mod auth;
mod config;
mod routes;

use axum::extract::FromRef;
use chronicle::db::MongoDb;
use futures::Future;
use hyper::{Method, Server};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

use self::routes::routes;
pub use self::{
    config::{ApiConfig, ApiData},
    error::{ApiError, ApiResult, AuthError, ConfigError},
    secret_key::SecretKey,
};

pub const DEFAULT_PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, Default)]
pub struct RegisteredRoutes(Vec<String>);

impl RegisteredRoutes {
    pub fn register(&mut self, route: impl Into<String>) -> String {
        let str = route.into();
        self.0.push(str.clone());
        str
    }

    pub fn list(&self) -> &[String] {
        self.0.as_slice()
    }
}

#[derive(Debug, Clone, FromRef)]
pub struct ApiState {
    db: MongoDb,
    api_data: ApiData,
    routes: RegisteredRoutes,
}

/// The Chronicle API actor
#[derive(Debug, Clone)]
pub struct ApiWorker {
    state: ApiState,
}

impl ApiWorker {
    /// Create a new Chronicle API actor from a mongo connection.
    pub fn new(db: &MongoDb, config: &ApiConfig) -> Result<Self, ConfigError> {
        Ok(Self {
            state: ApiState {
                db: db.clone(),
                api_data: config.clone().try_into()?,
                routes: Default::default(),
            },
        })
    }

    pub async fn run(&self, shutdown_handle: impl Future<Output = ()>) -> eyre::Result<()> {
        info!("Starting API server on port `{}`", self.state.api_data.port);

        let mut state = self.state.clone();

        let port = self.state.api_data.port;
        let routes = routes(&mut state)
            .with_state(state)
            .layer(CatchPanicLayer::new())
            .layer(TraceLayer::new_for_http())
            .layer(
                CorsLayer::new()
                    .allow_origin(self.state.api_data.allow_origins.clone())
                    .allow_methods(vec![Method::GET, Method::OPTIONS])
                    .allow_headers(Any)
                    .allow_credentials(false),
            );

        Server::bind(&([0, 0, 0, 0], port).into())
            .serve(routes.into_make_service())
            .with_graceful_shutdown(shutdown_handle)
            .await?;

        Ok(())
    }
}
