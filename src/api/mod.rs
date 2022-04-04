// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Contains routes that can be used to access data stored by Chronicle
//! as well as the health of the application, metrics, and analytics.

mod error;
mod extractors;
#[cfg(feature = "api-metrics")]
mod metrics;
mod responses;
mod routes;

use actix::{Actor, ActorContext, Context, Handler, Message};
use axum::Server;
use error::ListenerError;
use mongodb::{options::ClientOptions, Client};
use routes::routes;
use tokio::sync::oneshot;

use crate::config::mongo::MongoConfig;

/// The Chronicle API actor
#[derive(Debug)]
pub struct ChronicleAPI {
    client: Client,
    server_handle: Option<oneshot::Sender<()>>,
}

impl ChronicleAPI {
    /// Create a new Chronicle API actor from a mongo connection config.
    /// Will fail if the config is invalid.
    pub fn new(mongo_config: MongoConfig) -> anyhow::Result<Self> {
        #[cfg(feature = "api-metrics")]
        {
            metrics::register_metrics();
        }
        let client_opts: ClientOptions = mongo_config.into();
        log::info!("Connecting to MongoDB");
        let client = Client::with_options(client_opts)?;
        Ok(Self {
            client,
            server_handle: None,
        })
    }
}

impl Actor for ChronicleAPI {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let db = self.client.database("permanode");
        let (sender, receiver) = oneshot::channel();
        log::info!("Starting Axum server");
        tokio::spawn(async {
            Server::bind(&([0, 0, 0, 0], 8000).into())
                .serve(routes(db).into_make_service())
                .with_graceful_shutdown(shutdown_signal(receiver))
                .await
                .unwrap();
        });
        self.server_handle = Some(sender);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let Some(handle) = self.server_handle.take() {
            log::info!("Stopping Axum server");
            handle.send(()).unwrap();
        }
        log::info!("Stopping API");
    }
}

/// An indicator message that can be sent to the Chronicle API actor to shut it down gracefully
#[derive(Message)]
#[rtype(result = "()")]
pub struct ShutdownAPI;

impl Handler<ShutdownAPI> for ChronicleAPI {
    type Result = ();

    fn handle(&mut self, _msg: ShutdownAPI, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

async fn shutdown_signal(recv: oneshot::Receiver<()>) {
    if let Err(e) = recv.await {
        log::error!("Error receiving shutdown signal: {}", e);
    }
}
