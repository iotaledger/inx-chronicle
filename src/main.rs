// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use backstage::prelude::*;
use chronicle::{db, error::Error};
use futures::{FutureExt, StreamExt};
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    Status,
};
use log::{debug, info};
use mongodb::{
    bson,
    bson::{doc, Document},
    options::{ClientOptions, Credential},
    Client, Database,
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

#[derive(Debug)]
/// Supervisor actor
pub struct Launcher;

#[async_trait]
impl Actor for Launcher {
    const PATH: &'static str = "launcher";

    type Data = ();

    type Context = UnsupervisedContext<Self>;

    async fn init(&mut self, cx: &mut Self::Context) -> Result<Self::Data, ActorError>
    where
        Self: 'static + Sized + Send + Sync,
    {
        let mut retries = 3;
        loop {
            let res = cx.spawn_actor(INXListener).await;
            if res.is_err() {
                if retries > 0 {
                    retries -= 1;
                } else {
                    Err(anyhow::anyhow!("Failed to spawn INXListener"))?;
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<StatusChange<INXListener>> for Launcher {
    async fn handle_event(
        &mut self,
        _cx: &mut Self::Context,
        _event: StatusChange<INXListener>,
        _data: &mut Self::Data,
    ) -> Result<(), ActorError> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<INXListener>> for Launcher {
    async fn handle_event(
        &mut self,
        cx: &mut Self::Context,
        mut event: Report<INXListener>,
        _data: &mut Self::Data,
    ) -> Result<(), ActorError> {
        match event.as_mut() {
            Err(e) => {
                log::error!("{:?}", e.error);
                match e.error.request.as_mut() {
                    Some(req) => match req {
                        ActorRequest::Restart(d) => match d.take() {
                            Some(d) => {
                                let handle = cx.handle().clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(d).await;
                                    handle.send(event).ok();
                                });
                            }
                            None => {
                                cx.spawn_actor(INXListener).await?;
                            }
                        },
                        _ => (),
                    },
                    None => (),
                }
            }
            Ok(_) => (),
        }
        Ok(())
    }
}

/// A worker that writes messages from [`inx`] to the database.
#[derive(Debug)]
pub struct INXListener;

#[async_trait]
impl Actor for INXListener {
    const PATH: &'static str = "inx_listener";

    type Data = Res<Database>;

    type Context = SupervisedContext<Self, Launcher, Act<Launcher>>;

    async fn init(&mut self, cx: &mut Self::Context) -> Result<Self::Data, ActorError>
    where
        Self: 'static + Sized + Send + Sync,
    {
        info!("Connecting to INX...");
        match InxClient::connect("http://localhost:9029").await {
            Ok(mut inx_client) => {
                info!("Connected to INX.");
                let response = inx_client.read_node_status(NoParams {}).await.map_err(|e| anyhow!(e))?;
                info!("Node status: {:#?}", response.into_inner());
                let mut message_stream = inx_client
                    .listen_to_messages(MessageFilter {})
                    .await
                    .map_err(|e| anyhow!(e))?
                    .into_inner();
                info!("Subscribed to `ListenToMessages`.");

                let handle = cx.handle().clone();
                cx.spawn_task(move |_| {
                    async move {
                        while let Some(msg) = message_stream.next().await {
                            handle.send(msg)?;
                        }
                        Ok(())
                    }
                    .boxed()
                })
                .await;

                let mut milestone_stream = inx_client
                    .listen_to_latest_milestone(NoParams {})
                    .await
                    .map_err(|e| anyhow!(e))?
                    .into_inner();
                info!("Subscribed to `ListenToLatestMilestone`.");

                let handle = cx.handle().clone();
                cx.spawn_task(move |_| {
                    async move {
                        while let Some(msg) = milestone_stream.next().await {
                            handle.send(msg)?;
                        }
                        Ok(())
                    }
                    .boxed()
                })
                .await;
            }
            Err(e) => {
                return Err(ActorError {
                    source: anyhow!("Could not connect to INX: {}", e),
                    request: ActorRequest::Restart(Some(Duration::from_secs(3))).into(),
                });
            }
        }
        Ok(cx.link_data::<Res<Database>>().await?)
    }
}

#[async_trait]
impl HandleEvent<Result<inx::proto::Message, Status>> for INXListener {
    async fn handle_event(
        &mut self,
        _cx: &mut Self::Context,
        inx_msg: Result<inx::proto::Message, Status>,
        db: &mut Self::Data,
    ) -> Result<(), ActorError> {
        match inx_msg {
            Ok(inx_msg) => {
                let message_id = inx_msg.message_id.ok_or_else(|| anyhow!("No message id"))?;
                debug!("Received message from INX: {:?}", message_id);
                // TODO: Get rid of unwraps
                let message_id = &message_id.id;
                let message = &inx_msg.message.ok_or_else(|| anyhow!("No message bytes"))?.data;

                db.0.collection::<Document>(db::collections::stardust::raw::MESSAGES)
                    .insert_one(
                        doc! {
                            "message_id": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message_id.clone()},
                            "raw_message": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message.clone()},
                        },
                        None,
                    )
                    .await.map_err(|e| anyhow!(e))?;
                Ok(())
            }
            Err(e) => Err(ActorError {
                source: anyhow!(e),
                request: ActorRequest::Restart(Some(Duration::from_secs(3))).into(),
            }),
        }
    }
}

#[async_trait]
impl HandleEvent<Result<inx::proto::Milestone, Status>> for INXListener {
    async fn handle_event(
        &mut self,
        _cx: &mut Self::Context,
        inx_milestone: Result<inx::proto::Milestone, Status>,
        db: &mut Self::Data,
    ) -> Result<(), ActorError> {
        match inx_milestone {
            Ok(inx_milestone) => {
                info!("Received milestone from INX: {:?}", inx_milestone.milestone_index);
                // TODO: Get rid of unwraps
                let milestone_index = inx_milestone.milestone_index;
                let milestone_timestamp = inx_milestone.milestone_timestamp;
                let message_id = &inx_milestone.message_id.ok_or_else(|| anyhow!("No message id"))?.id;

                db.0.collection::<Document>(db::collections::stardust::MILESTONES)
                .insert_one(
                    doc! {
                        "milestone_index": bson::to_bson(&milestone_index).map_err(|e| anyhow!(e))?,
                        "milestone_timestamp": bson::to_bson(&milestone_timestamp).map_err(|e| anyhow!(e))?,
                        "message_id": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message_id.clone()},
                    },
                    None,
                )
                .await.map_err(|e| anyhow!(e))?;
                Ok(())
            }
            Err(e) => Err(ActorError {
                source: anyhow!(e),
                request: ActorRequest::Restart(Some(Duration::from_secs(3))).into(),
            }),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();
    std::panic::set_hook(Box::new(|info| {
        log::error!("{}", info);
    }));

    RuntimeScope::launch(|scope| {
        async move {
            let db = connect_database().await?;

            scope.add_resource(db).await;

            let launcher_handle = scope.spawn_actor_unsupervised(Launcher).await?;

            tokio::signal::ctrl_c().await.map_err(|_| Error::ShutdownFailed)?;
            launcher_handle.shutdown().await;
            Ok(())
        }
        .boxed()
    })
    .await?;

    Ok(())
}
