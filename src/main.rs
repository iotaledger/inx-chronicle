// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, System, WrapFuture};
use chronicle::{db, error::Error};
use futures::stream::StreamExt;
use inx::{client::InxClient, proto::MessageFilter};
use log::{debug, info};
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};
use tonic::transport::Channel;

async fn messages(client: &mut InxClient<Channel>, writer: Addr<WriterWorker>) {
    let response = client.listen_to_messages(MessageFilter {}).await;
    info!("Subscribed to `ListenToMessages`.");
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        debug!("INX received message.");
        if let Ok(msg) = item {
            writer.send(InxMessage(msg)).await.unwrap();
        }
    }
}

async fn connect_database<S: AsRef<str>>(location: S) -> Result<mongodb::Database, Error> {
    let mut client_options = ClientOptions::parse(location).await?;
    client_options.app_name = Some("Chronicle".to_string());
    let client = Client::with_options(client_options)?;
    Ok(client.database(db::DB_NAME))
}

/// A worker that writes messages from [`inx`] to the database.
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
        info!("WriterWorker started.");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("WriterWorker stopped.");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct InxMessage(inx::proto::Message);

impl Handler<InxMessage> for WriterWorker {
    type Result = ();

    fn handle(&mut self, inx_msg: InxMessage, ctx: &mut Self::Context) -> Self::Result {
        let db = self.db.clone();
        let fut = Box::pin(async move {
            // TODO: Get rid of unwraps
            let message_id_str = String::from_utf8_lossy(&inx_msg.0.message_id.unwrap().id).into_owned();
            let message_str = String::from_utf8_lossy(&inx_msg.0.message.unwrap().data).into_owned();

            db.collection::<Document>(db::collections::STARDUST_MESSAGES)
                .insert_one(
                    doc! {
                        "message_id": message_id_str, "raw_message": message_str
                    },
                    None,
                )
                .await
                .unwrap();
        });

        let actor_fut = fut.into_actor(self);
        ctx.wait(actor_fut);
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

        let inx_worker_addr = WriterWorker::new(db).start();
        let c = inx_worker_addr.clone();

        tokio::spawn(async {
            let mut inx_client = InxClient::connect("http://localhost:9029").await.unwrap();
            messages(&mut inx_client, c).await;
        });

        tokio::signal::ctrl_c().await.map_err(|_| Error::ShutdownFailed)?;
        inx_worker_addr.send(ShutdownMessage).await.unwrap();
        Ok(())
    });

    Ok(())
}
