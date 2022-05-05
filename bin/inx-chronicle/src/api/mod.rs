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
mod routes;

use async_trait::async_trait;
use axum::Server;
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{context::ActorContext, Actor},
        spawn_task,
    },
};
pub use error::ApiError;
pub(crate) use responses::impl_success_response;
pub use responses::SuccessBody;
use routes::routes;
use tokio::{sync::oneshot, task::JoinHandle};

/// The result of a request to the api
pub type ApiResult<T> = Result<T, ApiError>;

/// The Chronicle API actor
#[derive(Debug)]
pub struct ApiWorker {
    db: MongoDb,
    server_handle: Option<(JoinHandle<hyper::Result<()>>, oneshot::Sender<()>)>,
}

impl ApiWorker {
    /// Create a new Chronicle API actor from a mongo connection.
    pub fn new(db: MongoDb) -> Self {
        Self {
            db,
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
        let db = self.db.clone();
        let api_handle = cx.handle().clone();
        let join_handle = spawn_task("Axum server", async move {
            let res = Server::bind(&([0, 0, 0, 0], 9092).into())
                .serve(routes(db).into_make_service())
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
