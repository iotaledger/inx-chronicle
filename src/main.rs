// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

use std::fmt::Debug;

use chronicle::{db, error::Error};
use derive_more::From;
use futures::{
    future::{AbortHandle, Abortable},
    Future, StreamExt,
};
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    Channel, Status,
};
use log::{debug, error, info};
use mongodb::{
    bson,
    bson::{doc, Document},
    options::{ClientOptions, Credential},
    Client, Database,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

async fn connect_database() -> Result<Database, Error> {
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

/// Messages result wrapper
#[derive(Debug, From)]
pub struct MessagesResult(Result<(), Error>);

async fn messages(mut client: InxClient<Channel>, handle: UnboundedSender<INXListenerEvent>) -> Result<(), Error> {
    let mut stream = client.listen_to_messages(MessageFilter {}).await?.into_inner();
    info!("Subscribed to `ListenToMessages`.");

    while let Some(msg) = stream.next().await {
        debug!("INX received message.");
        handle
            .send(INXListenerEvent::Message(msg))
            .map_err(|_| Error::other("Failed to send message to listener."))?;
    }
    Ok(())
}

/// Milestones result wrapper
#[derive(Debug, From)]
pub struct MilestonesResult(Result<(), Error>);

async fn latest_milestone(
    mut client: InxClient<Channel>,
    handle: UnboundedSender<INXListenerEvent>,
) -> Result<(), Error> {
    let mut stream = client.listen_to_latest_milestone(NoParams {}).await?.into_inner();
    info!("Subscribed to `ListenToLatestMilestone`.");

    while let Some(msg) = stream.next().await {
        debug!("INX received latest milestone.");
        handle
            .send(INXListenerEvent::Milestone(msg))
            .map_err(|_| Error::other("Failed to send milestone to listener."))?;
    }
    Ok(())
}

fn supervise<
    E: 'static + From<R> + Debug + Send,
    R: From<F::Output> + Send,
    F: 'static + Send + Future<Output = Result<(), Error>>,
>(
    handle: UnboundedSender<E>,
    actor: F,
) -> AbortHandle {
    let (abort_handle, abort_reg) = AbortHandle::new_pair();
    tokio::spawn(async move {
        let res = match Abortable::new(actor, abort_reg).await {
            Ok(res) => res,
            Err(e) => Err(e.into()),
        };
        handle.send(E::from(R::from(res))).unwrap();
    });
    abort_handle
}

#[derive(Debug, From)]
pub enum INXListenerEvent {
    Message(Result<inx::proto::Message, Status>),
    MessagesResult(MessagesResult),
    Milestone(Result<inx::proto::Milestone, Status>),
    MilestonesResult(MilestonesResult),
    Shutdown,
}

/// A worker that writes messages from [`inx`] to the database.
pub struct INXListener {
    db: mongodb::Database,
    handle: UnboundedSender<INXListenerEvent>,
    inbox: UnboundedReceiver<INXListenerEvent>,
}

impl INXListener {
    fn new(db: mongodb::Database) -> Self {
        let (handle, inbox) = tokio::sync::mpsc::unbounded_channel();
        Self { db, handle, inbox }
    }

    fn handle(&self) -> UnboundedSender<INXListenerEvent> {
        self.handle.clone()
    }

    async fn start(mut self) -> Result<(), Error> {
        info!("INXListener started.");
        info!("Connecting to INX...");
        let mut inx_client = InxClient::connect("http://localhost:9029")
            .await
            .map_err(|e| Error::other(e.to_string()))?;
        info!("Connected to INX.");
        let response = inx_client.read_node_status(NoParams {}).await;
        info!(
            "Node status: {:#?}",
            response.map_err(|e| Error::other(e.to_string()))?.into_inner()
        );

        let messages_abort = supervise::<INXListenerEvent, MessagesResult, _>(
            self.handle(),
            messages(inx_client.clone(), self.handle()),
        );
        let milestones_abort = supervise::<INXListenerEvent, MilestonesResult, _>(
            self.handle(),
            latest_milestone(inx_client.clone(), self.handle()),
        );
        while let Some(evt) = self.inbox.recv().await {
            match evt {
                INXListenerEvent::Message(msg) => {
                    self.handle_message(msg?).await?;
                }
                INXListenerEvent::MessagesResult(MessagesResult(res)) => {
                    if let Err(err) = res {
                        error!("Error while listening to messages: {}", err);
                        if self.handle.send(INXListenerEvent::Shutdown).is_err() {
                            break;
                        }
                    }
                }
                INXListenerEvent::Milestone(msg) => {
                    self.handle_milestone(msg?).await?;
                }
                INXListenerEvent::MilestonesResult(MilestonesResult(res)) => {
                    if let Err(err) = res {
                        error!("Error while listening to milestones: {}", err);
                        if self.handle.send(INXListenerEvent::Shutdown).is_err() {
                            break;
                        }
                    }
                }
                INXListenerEvent::Shutdown => break,
            }
        }
        // This isn't really good enough, due to early exits :(
        messages_abort.abort();
        milestones_abort.abort();
        Ok(())
    }

    async fn handle_message(&mut self, inx_msg: inx::proto::Message) -> Result<(), Error> {
        let message_id = inx_msg.message_id.ok_or_else(|| Error::other("No message id"))?;
        debug!("Received message from INX: {:?}", message_id);
        let message_id = &message_id.id;
        let message = &inx_msg.message.ok_or_else(|| Error::other("No message bytes"))?.data;

        self.db
            .collection::<Document>(db::collections::stardust::raw::MESSAGES)
            .insert_one(
                doc! {
                    "message_id": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message_id.clone()},
                    "raw_message": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message.clone()},
                },
                None,
            )
            .await?;
        Ok(())
    }

    async fn handle_milestone(&mut self, inx_milestone: inx::proto::Milestone) -> Result<(), Error> {
        info!("Received milestone from INX: {:?}", inx_milestone.milestone_index);
        let milestone_index = inx_milestone.milestone_index;
        let milestone_timestamp = inx_milestone.milestone_timestamp;
        let message_id = &inx_milestone
            .message_id
            .ok_or_else(|| Error::other("No message id"))?
            .id;

        self.db
            .collection::<Document>(db::collections::stardust::MILESTONES)
            .insert_one(
                doc! {
                    "milestone_index": bson::to_bson(&milestone_index)?,
                    "milestone_timestamp": bson::to_bson(&milestone_timestamp)?,
                    "message_id": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message_id.clone()},
                },
                None,
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, From)]
pub struct INXListenerResult(Result<(), Error>);

#[derive(Debug, From)]
pub enum LauncherEvent {
    INXListenerResult(INXListenerResult),
    Shutdown,
}

pub struct Launcher {
    handle: UnboundedSender<LauncherEvent>,
    inbox: UnboundedReceiver<LauncherEvent>,
}

impl Launcher {
    fn new() -> Self {
        let (handle, inbox) = tokio::sync::mpsc::unbounded_channel();
        Self { handle, inbox }
    }

    fn handle(&self) -> UnboundedSender<LauncherEvent> {
        self.handle.clone()
    }

    async fn start(mut self, db: Database) -> Result<(), Error> {
        let inx_listener = INXListener::new(db);
        let listener_handle = inx_listener.handle();
        let listener_abort = supervise::<LauncherEvent, INXListenerResult, _>(self.handle(), inx_listener.start());
        while let Some(evt) = self.inbox.recv().await {
            match evt {
                LauncherEvent::INXListenerResult(INXListenerResult(res)) => {
                    if let Err(err) = res {
                        error!("Error while listening to INX: {}", err);
                        if self.handle.send(LauncherEvent::Shutdown).is_err() {
                            break;
                        }
                    }
                }
                LauncherEvent::Shutdown => {
                    if listener_handle.send(INXListenerEvent::Shutdown).is_err() {
                        listener_abort.abort();
                    }
                    break;
                }
            }
        }
        listener_abort.abort();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let db = connect_database().await?;
    let launcher = Launcher::new();
    let launcher_handle = launcher.handle();

    tokio::select! {
        _ = launcher.start(db) => {},
        r = tokio::signal::ctrl_c() => {
            r.map_err(|_| Error::ShutdownFailed)?;
            launcher_handle.send(LauncherEvent::Shutdown).ok();
        },
    }

    Ok(())
}
