// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

use std::net::SocketAddr;
use std::sync::Arc;

use actix::{Actor, ActorContext, Context, Handler, Message, System};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use futures::stream::StreamExt;
use inx::{
    client::InxClient,
    proto::{ApiRouteRequest, MessageFilter},
};
use mongodb::bson::{doc, to_bson, Document};
use mongodb::{options::ClientOptions, Client};

use tokio::sync::RwLock;
use tonic::transport::Channel;

use chronicle::error::Error;

use log::{debug, info};

const DB_NAME: &str = "chronicle-test";
const STARDUST_MESSAGES: &str = "stardust_messages";

async fn messages(client: &mut InxClient<Channel>, db: mongodb::Database) {
    let response = client.listen_to_messages(MessageFilter {}).await;
    println!("{:#?}", response);
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        debug!("received message");
        let collection = db.collection::<Document>(STARDUST_MESSAGES);
        if let Ok(inx::proto::Message {
            message_id: Some(msg_id),
            message: Some(message),
        }) = item
        {}
    }
    // stream is droped here and the disconnect info is send to server
}

async fn connect_database<S: AsRef<str>>(location: S) -> Result<mongodb::Database, Error> {
    let mut client_options = ClientOptions::parse(location).await?;
    client_options.app_name = Some("Chronicle".to_string());
    let client = Client::with_options(client_options)?;
    Ok(client.database(DB_NAME))
}

pub struct WriterWorker {
    db: mongodb::Database,
}

impl WriterWorker {
    fn new(db: mongodb::Database) -> Self {
        Self { db }
    }
}

impl Actor for WriterWorker {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("InxWorker started.");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("InxWorker stopped.");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct InxMessage(inx::proto::Message);

impl Handler<InxMessage> for WriterWorker {
    type Result = ();

    fn handle(&mut self, inx_msg: InxMessage, ctx: &mut Self::Context) -> Self::Result {
        // TODO: Get rid of unwraps
        let message_id_str = String::from_utf8_lossy(&inx_msg.0.message_id.unwrap().id).into_owned();
        let message_str = String::from_utf8_lossy(&inx_msg.0.message.unwrap().data).into_owned();

        self.db
            .collection::<Document>(STARDUST_MESSAGES)
            .insert_one(
                doc! {
                    "message_id": message_id_str, "raw_message": message_str
                },
                None,
            )
            .await
            .unwrap();
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ShutdownMessage;

impl Handler<ShutdownMessage> for WriterWorker {
    type Result = ();

    fn handle(&mut self, _msg: ShutdownMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;
    env_logger::init();

    let system = System::new();

    let _: Result<(), Error> = system.block_on(async {
        let db = connect_database("mongodb://localhost:27017").await?;
        let mut inx_client = InxClient::connect("http://localhost:9029").await.unwrap();

        let inx_worker_addr = WriterWorker::new(db, inx_client).start();
        tokio::signal::ctrl_c().await.map_err(|_| Error::ShutdownFailed)?;
        inx_worker_addr.send(ShutdownMessage).await.unwrap();
        Ok(())
    });

    Ok(())
}
