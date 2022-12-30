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
pub mod config;
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
    config::{ApiConfig, ApiConfigData},
    error::{ApiError, ApiResult, AuthError, ConfigError},
    secret_key::SecretKey,
};

pub const DEFAULT_PAGE_SIZE: usize = 100;

/// The Chronicle API actor
#[derive(Debug, Clone, FromRef)]
pub struct ApiWorker {
    db: MongoDb,
    api_data: ApiConfigData,
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

        let port = self.api_data.port;
        let routes = routes(self.api_data.clone())
            .with_state(self.clone())
            .layer(CatchPanicLayer::new())
            .layer(TraceLayer::new_for_http())
            .layer(
                CorsLayer::new()
                    .allow_origin(self.api_data.allow_origins.clone())
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
