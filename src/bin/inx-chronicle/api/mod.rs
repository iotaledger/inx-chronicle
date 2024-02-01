// Copyright 2023 IOTA Stiftung
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
// #[cfg(feature = "poi")]
// mod poi;
mod router;
mod routes;

use std::sync::Arc;

use axum::extract::FromRef;
use chronicle::db::MongoDb;
use futures::Future;
use hyper::Method;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

use self::router::RouteNode;
pub use self::{
    config::{ApiConfig, ApiConfigData},
    error::{ApiError, ApiResult, AuthError},
    secret_key::SecretKey,
};

pub const DEFAULT_PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, FromRef)]
pub struct ApiState {
    db: MongoDb,
    api_data: Arc<ApiConfigData>,
    routes: Arc<RouteNode>,
}

/// The Chronicle API actor
#[derive(Default, Clone, Debug)]
pub struct ApiWorker;

impl ApiWorker {
    /// Run the API with a provided mongodb connection and config.
    pub async fn run(
        db: MongoDb,
        config: ApiConfig,
        shutdown_handle: impl Future<Output = ()> + Send + 'static,
    ) -> eyre::Result<()> {
        let api_data = Arc::new(ApiConfigData::try_from(config)?);
        info!("Starting API server on port `{}`", api_data.port);

        let port = api_data.port;
        let router = routes::routes(api_data.clone())
            .layer(CatchPanicLayer::new())
            .layer(TraceLayer::new_for_http())
            .layer(
                CorsLayer::new()
                    .allow_origin(api_data.allow_origins.clone())
                    .allow_methods(vec![Method::GET, Method::OPTIONS])
                    .allow_headers(Any)
                    .allow_credentials(false),
            );

        let (routes, router) = router.finish();

        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
        axum::serve(
            listener,
            router
                .with_state(ApiState {
                    db,
                    api_data,
                    routes: Arc::new(routes),
                })
                .into_make_service(),
        )
        .with_graceful_shutdown(shutdown_handle)
        .await?;

        Ok(())
    }
}
