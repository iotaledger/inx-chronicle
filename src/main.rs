// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

use actix::{
    Actor, ActorContext, ActorFutureExt, AsyncContext, Context, Handler, Message, StreamHandler, System, WrapFuture,
};
use chronicle::{db, error::Error};
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    Status,
};
use log::{debug, error, info};
use mongodb::{
    bson,
    bson::{doc, Document},
    options::{ClientOptions, Credential},
    Client,
};

async fn connect_database() -> Result<mongodb::Database, Error> {
    let client_options = ClientOptions::builder()
        .credential(
            Credential::builder()
                .username("root".to_string())
                .password("pass".to_string())
                .build(),
        )
        .app_name("Chronicle".to_string())
        .build();
    let client = Client::with_options(client_options)?;
    Ok(client.database(db::DB_NAME))
}

/// A worker that writes messages from [`inx`] to the database.
pub struct INXListener {
    db: mongodb::Database,
}

impl INXListener {
    fn new(db: mongodb::Database) -> Self {
        Self { db }
    }
}

impl Actor for INXListener {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("INXListener started.");
        let fut = Box::pin(
            async move {
                info!("Connecting to INX...");
                match InxClient::connect("http://localhost:9029").await {
                    Ok(mut inx_client) => {
                        info!("Connected to INX.");
                        let response = inx_client.read_node_status(NoParams {}).await;
                        info!("Node status: {:#?}", response.unwrap().into_inner());
                        let response = inx_client.listen_to_messages(MessageFilter {}).await;
                        info!("Subscribed to `ListenToMessages`.");
                        let message_stream = response.unwrap().into_inner();

                        let response = inx_client.listen_to_latest_milestone(NoParams {}).await;
                        info!("Subscribed to `ListenToLatestMilestone`.");
                        let milestone_stream = response.unwrap().into_inner();

                        Ok((message_stream, milestone_stream))
                    }
                    Err(e) => {
                        error!("Could not connect to INX: {}", e);
                        Err(e)
                    }
                }
            }
            .into_actor(self)
            .map(|streams, _, ctx| match streams {
                Ok((message_stream, milestone_stream)) => {
                    <Self as StreamHandler<Result<inx::proto::Message, _>>>::add_stream(message_stream, ctx);
                    <Self as StreamHandler<Result<inx::proto::Milestone, _>>>::add_stream(milestone_stream, ctx);
                }
                Err(_) => {
                    ctx.terminate();
                }
            }),
        );
        ctx.wait(fut);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("INXListener stopped.");
    }
}

impl StreamHandler<Result<inx::proto::Message, Status>> for INXListener {
    fn handle(&mut self, inx_msg: Result<inx::proto::Message, Status>, ctx: &mut Self::Context) {
        if let Ok(inx_msg) = inx_msg {
            let message_id = inx_msg.message_id.unwrap();
            debug!("Received message from INX: {:?}", message_id);
            let db = self.db.clone();
            let fut = Box::pin(async move {
                // TODO: Get rid of unwraps
                let message_id = &message_id.id;
                let message = &inx_msg.message.unwrap().data;

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
}

impl StreamHandler<Result<inx::proto::Milestone, Status>> for INXListener {
    fn handle(&mut self, inx_milestone: Result<inx::proto::Milestone, Status>, ctx: &mut Self::Context) {
        if let Ok(inx_milestone) = inx_milestone {
            info!("Received milestone from INX: {:?}", inx_milestone.milestone_index);
            let db = self.db.clone();
            let fut = Box::pin(async move {
                // TODO: Get rid of unwraps
                let milestone_index = inx_milestone.milestone_index;
                let milestone_timestamp = inx_milestone.milestone_timestamp;
                let message_id = &inx_milestone.message_id.unwrap().id;

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
}

#[derive(Message)]
#[rtype(result = "()")]
struct ShutdownMessage;

impl Handler<ShutdownMessage> for INXListener {
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
        let db = connect_database().await?;

        let inx_listener_addr = INXListener::new(db).start();

        tokio::signal::ctrl_c().await.map_err(|_| Error::ShutdownFailed)?;

        inx_listener_addr.send(ShutdownMessage).await.unwrap();
        Ok(())
    });
    result?;

    Ok(())
}
