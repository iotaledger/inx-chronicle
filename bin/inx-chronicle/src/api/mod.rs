// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Contains routes that can be used to access data stored by Chronicle
//! as well as the health of the application and analytics.

mod extractors;

#[cfg(feature = "api-v2")]
mod v2;

mod error;
mod responses;
mod routes;

use async_trait::async_trait;
use axum::Server;
use chronicle::{
    db::MongoDatabase,
    runtime::actor::{context::ActorContext, Actor},
};
use routes::routes;
use serde::Deserialize;
use tokio::{sync::oneshot, task::JoinHandle};

pub use self::error::ApiError;

/// The result of a request to the api
pub type ApiResult<T> = Result<T, ApiError>;

/// API version enumeration
#[derive(Copy, Clone, Deserialize)]
pub enum ApiVersion {
    /// Stardust API version 2
    #[serde(rename = "v2")]
    V2,
}

/// The Chronicle API actor
#[derive(Debug)]
pub struct ApiWorker {
    db: MongoDatabase,
    server_handle: Option<(JoinHandle<hyper::Result<()>>, oneshot::Sender<()>)>,
}

impl ApiWorker {
    /// Create a new Chronicle API actor from a mongo connection.
    pub fn new(db: MongoDatabase) -> Self {
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
        let join_handle = tokio::spawn(async move {
            let res = Server::bind(&([0, 0, 0, 0], 8000).into())
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
