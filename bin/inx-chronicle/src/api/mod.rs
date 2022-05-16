// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Contains routes that can be used to access data stored by Chronicle
//! as well as the health of the application and analytics.

mod extractors;

#[cfg(feature = "stardust")]
pub(crate) mod stardust;

mod error;
#[macro_use]
mod responses;
mod config;
mod routes;

use async_trait::async_trait;
use axum::{Extension, Server};
use chronicle::{
    db::MongoDb,
    runtime::{spawn_task, Actor, ActorContext},
};
use hyper::Method;
use tokio::{sync::oneshot, task::JoinHandle};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};

pub use self::{config::ApiConfig, error::ApiError};
use self::{responses::impl_success_response, routes::routes};

/// The result of a request to the api
pub type ApiResult<T> = Result<T, ApiError>;

/// The Chronicle API actor
#[derive(Debug)]
pub struct ApiWorker {
    db: MongoDb,
    config: ApiConfig,
    server_handle: Option<(JoinHandle<hyper::Result<()>>, oneshot::Sender<()>)>,
}

impl ApiWorker {
    /// Create a new Chronicle API actor from a mongo connection.
    pub fn new(db: MongoDb, config: ApiConfig) -> Self {
        Self {
            db,
            config,
            server_handle: None,
        }
    }
}

#[async_trait]
impl Actor for ApiWorker {
    type State = ();

    type Error = ApiError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let (sender, receiver) = oneshot::channel();
        log::info!("Starting API server");
        let api_handle = cx.handle().clone();
        let port = self.config.port;
        let routes = routes()
            .layer(Extension(self.db.clone()))
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

    async fn shutdown(&mut self, cx: &mut ActorContext<Self>, _state: &mut Self::State) -> Result<(), Self::Error> {
        log::debug!("{} shutting down ({})", self.name(), cx.id());
        if let Some((join_handle, shutdown_handle)) = self.server_handle.take() {
            // Try to shut down axum. It may have already shut down, which is fine.
            shutdown_handle.send(()).ok();
            // Wait to shutdown until the child task is complete.
            // Unwrap: Failures to join on this handle can safely be propagated as panics via the runtime.
            join_handle.await.unwrap()?;
        }
        log::info!("Stopping API server");
        Ok(())
    }
}

async fn shutdown_signal(recv: oneshot::Receiver<()>) {
    if let Err(e) = recv.await {
        log::error!("Error receiving shutdown signal: {}", e);
    }
}
