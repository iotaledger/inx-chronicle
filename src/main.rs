// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, System, WrapFuture};
use chronicle::{db, error::Error};
use futures::StreamExt;
use inx::{client::InxClient, proto::MessageFilter, proto::NoParams, Channel};
use log::{debug, error, info};
use mongodb::{
    bson,
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};

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

async fn latest_milestone(client: &mut InxClient<Channel>, writer: Addr<WriterWorker>) {
    let response = client.listen_to_latest_milestone(NoParams {}).await;
    info!("Subscribed to `ListenToLatestMilestone`.");
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        debug!("INX received latest milestone.");
        if let Ok(milestone) = item {
            writer.send(InxMilestone(milestone)).await.unwrap();
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
            let message_id = &inx_msg.0.message_id.unwrap().id;
            let message = &inx_msg.0.message.unwrap().data;

            db.collection::<Document>(db::collections::stardust::raw::MESSAGES)
                .insert_one(
                    doc! {
                        "message_id": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message_id.clone()},
                        "raw_message": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message.clone()},
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
struct InxMilestone(inx::proto::Milestone);

impl Handler<InxMilestone> for WriterWorker {
    type Result = (); // TODO return error

    fn handle(&mut self, inx_milestone: InxMilestone, ctx: &mut Self::Context) -> Self::Result {
        let db = self.db.clone();
        let fut = Box::pin(async move {
            // TODO: Get rid of unwraps
            let milestone_index = inx_milestone.0.milestone_index;
            let milestone_timestamp = inx_milestone.0.milestone_timestamp;
            let message_id = &inx_milestone.0.message_id.unwrap().id;

            db.collection::<Document>(db::collections::stardust::MILESTONES)
                .insert_one(
                    doc! {
                        "milestone_index": bson::to_bson(&milestone_index).unwrap(),
                        "milestone_timestamp": bson::to_bson(&milestone_timestamp).unwrap(),
                        "message_id": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message_id.clone()},
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

    let result: Result<(), Error> = system.block_on(async {
        let db = connect_database("mongodb://localhost:27017").await?;

        let inx_worker_addr = WriterWorker::new(db).start();
        let c1 = inx_worker_addr.clone();
        let c2 = inx_worker_addr.clone();

        match InxClient::connect("http://localhost:9029").await {
            Ok(mut inx_client) => {
                let mut inx_c = inx_client.clone();

                tokio::spawn(async move {
                    messages(&mut inx_c, c1).await;
                });

                tokio::spawn(async move {
                    latest_milestone(&mut inx_client, c2).await;
                });

                tokio::signal::ctrl_c().await.map_err(|_| Error::ShutdownFailed)?;
            }
            Err(_) => {
                error!("Could not connect to INX.");
            }
        }
        
        inx_worker_addr.send(ShutdownMessage).await.unwrap();
        Ok(())
    });

    result?;
    Ok(())
}
