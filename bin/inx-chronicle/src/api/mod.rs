// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Contains routes that can be used to access data stored by Chronicle
//! as well as the health of the application and analytics.

mod extractors;

#[cfg(feature = "stardust")]
pub mod stardust;

mod error;
#[macro_use]
mod responses;
mod auth;
mod config;
#[cfg(feature = "metrics")]
mod metrics;
mod routes;

use async_trait::async_trait;
use axum::{Extension, Server};
use chronicle::{
    db::MongoDb,
    runtime::{spawn_task, Actor, ActorContext},
};
use hyper::Method;
use libp2p_core::identity::ed25519::SecretKey;
use tokio::{sync::oneshot, task::JoinHandle};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};

pub use self::{
    config::ApiConfig,
    error::{ApiError, ConfigError},
};
use self::{config::ApiData, responses::impl_success_response, routes::routes};

/// The result of a request to the api
pub type ApiResult<T> = Result<T, ApiError>;

/// The Chronicle API actor
#[derive(Debug)]
pub struct ApiWorker {
    db: MongoDb,
    config: ApiData,
    server_handle: Option<(JoinHandle<hyper::Result<()>>, oneshot::Sender<()>)>,
}

impl ApiWorker {
    /// Create a new Chronicle API actor from a mongo connection.
    pub fn new(db: &MongoDb, config: &ApiConfig, secret_key: &SecretKey) -> Result<Self, ConfigError> {
        Ok(Self {
            db: db.clone(),
            config: (config.clone(), secret_key.clone()).try_into()?,
            server_handle: None,
        })
    }
}

#[async_trait]
impl Actor for ApiWorker {
    type State = ();

    type Error = ApiError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let (sender, receiver) = oneshot::channel();
        log::info!("Starting API server on port `{}`", self.config.port);
        let api_handle = cx.handle().clone();
        let port = self.config.port;
        let routes = routes()
            .layer(Extension(self.db.clone()))
            .layer(Extension(self.config.clone()))
            .layer(CatchPanicLayer::new())
            .layer(TraceLayer::new_for_http())
            .layer(
                CorsLayer::new()
                    .allow_origin(
                        self.config
                            .allow_origins
                            .clone()
                            .map(AllowOrigin::try_from)
                            .transpose()?
                            .unwrap_or_else(AllowOrigin::any),
                    )
                    .allow_methods(vec![Method::GET, Method::OPTIONS])
                    .allow_headers(Any)
                    .allow_credentials(false),
            );

        #[cfg(feature = "metrics")]
        let routes = {
            use self::metrics::MetricsLayer;

            let layer = MetricsLayer::default();

            cx.metrics_registry().register(
                "incoming_requests",
                "Incoming API Requests",
                layer.metrics.incoming_requests.clone(),
            );

            routes.layer(layer)
        };

        let join_handle = spawn_task("Axum server", async move {
            let res = Server::bind(&([0, 0, 0, 0], port).into())
                .serve(routes.into_make_service())
                .with_graceful_shutdown(shutdown_signal(receiver))
                .await;
            // If the Axum server shuts down, we should also shutdown the API actor
            api_handle.shutdown();
            res
        });
        self.server_handle = Some((join_handle, sender));
        Ok(())
    }

    async fn shutdown(
        &mut self,
        cx: &mut ActorContext<Self>,
        _state: &mut Self::State,
        run_result: Result<(), Self::Error>,
    ) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());
        if let Some((join_handle, shutdown_handle)) = self.server_handle.take() {
            // Try to shut down axum. It may have already shut down, which is fine.
            shutdown_handle.send(()).ok();
            // Wait to shutdown until the child task is complete.
            // Unwrap: Failures to join on this handle can safely be propagated as panics via the runtime.
            join_handle.await.unwrap()?;
        }
        log::info!("Stopping API server");
        run_result
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "API Worker".into()
    }
}

async fn shutdown_signal(recv: oneshot::Receiver<()>) {
    if let Err(e) = recv.await {
        log::error!("Error receiving shutdown signal: {}", e);
    }
}
