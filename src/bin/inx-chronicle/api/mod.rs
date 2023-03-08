// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Contains routes that can be used to access data stored by Chronicle
//! as well as the health of the application and analytics.

mod error;
mod extractors;
mod secret_key;
#[macro_use]
mod responses;
mod auth;
pub mod config;
mod core;
mod explorer;
mod indexer;
#[cfg(feature = "poi")]
mod poi;
mod routes;

use axum::{extract::FromRef, Server};
use chronicle::db::MongoDb;
use futures::Future;
use hyper::Method;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

pub use self::{
    config::{ApiConfig, ApiConfigData},
    error::{ApiError, ApiResult, AuthError, ConfigError},
    secret_key::SecretKey,
};

pub const DEFAULT_PAGE_SIZE: usize = 100;

/// The Chronicle API actor
#[derive(Debug)]
pub struct ApiWorker {
    db: MongoDb,
    api_data: ApiConfigData,
}

// Our top level state that contains an `HttpClient` and a `Database`
//
// `#[derive(FromRef)]` makes them sub states so they can be extracted
// independently
#[derive(Clone, FromRef)]
pub struct AppState {
    db: MongoDb,
    config: ApiConfigData,
}

impl ApiWorker {
    /// Create a new Chronicle API actor from a mongo connection.
    pub fn new(db: &MongoDb, config: &ApiConfig) -> Result<Self, ConfigError> {
        Ok(Self {
            db: db.clone(),
            api_data: config.clone().try_into()?,
        })
    }

    pub async fn run(&self, shutdown_handle: impl Future<Output = ()>) -> eyre::Result<()> {
        info!("Starting API server on port `{}`", self.api_data.port);

        let state = AppState {
            db: self.db.clone(),
            config: self.api_data.clone(),
        };

        let routes = routes::routes(state.clone())
            .layer(CatchPanicLayer::new())
            .layer(TraceLayer::new_for_http())
            .layer(
                CorsLayer::new()
                    .allow_origin(self.api_data.allow_origins.clone())
                    .allow_methods(vec![Method::GET, Method::OPTIONS])
                    .allow_headers(Any)
                    .allow_credentials(false),
            )
            .with_state(state);

        let port = self.api_data.port;
        Server::bind(&([0, 0, 0, 0], port).into())
            .serve(routes.into_make_service())
            .with_graceful_shutdown(shutdown_handle)
            .await?;

        Ok(())
    }
}
