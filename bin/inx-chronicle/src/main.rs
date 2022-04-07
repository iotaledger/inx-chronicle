// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

mod cli;
mod config;

use chronicle::db::{MongoConfig, MongoDatabase};
use clap::Parser;
use futures::StreamExt;
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    Channel,
};
use log::{debug, error, warn, info};
use tokio::{sync::mpsc, task::JoinHandle};

use self::cli::CliArgs;

async fn subscribe_messages(client: &mut InxClient<Channel>, broker_addr: BrokerAddr) {
    let response = client.listen_to_messages(MessageFilter {}).await;
    info!("Subscribed to `ListenToMessages`.");
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        let msg = item.unwrap();
        debug!("INX received message.");
        broker_addr.sender.send(InxEvent::Message);
        // if let Err(err) = db.insert_message_raw(msg).await {
        //     error!("Failed to insert raw message: {err}");
        // }
    }
}

async fn subscribe_latest_milestone(client: &mut InxClient<Channel>, broker_addr: BrokerAddr) {
    let response = client.listen_to_latest_milestone(NoParams {}).await;
    info!("Subscribed to `ListenToLatestMilestone`.");
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        let milestone = item.unwrap();
        debug!("INX received latest milestone: {}", milestone.milestone_index);
        broker_addr.sender.send(InxEvent::Milestone);
        // if let Err(err) = db.insert_milestone(milestone).await {
        //     error!("Failed to insert milestone: {err}");
        // }
    }
}

enum InxEvent {
    Message,
    Milestone,
}

struct InxListener {
    // TODO: Consider storing `inx_client` to potentially restart some streams.
    message_handle: JoinHandle<()>,
    latest_milestone_handle: JoinHandle<()>,
}

struct Broker {
    _db: MongoDatabase,
    receiver: mpsc::UnboundedReceiver<InxEvent>,
}

impl Broker {
    fn new(db: MongoDatabase) -> BrokerAddr {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut broker = Broker { receiver, _db: db };
        tokio::spawn(async move { broker.run().await });
        BrokerAddr { sender }
    }

    async fn handle_event(&mut self, event: InxEvent) {
        match event {
            InxEvent::Message => warn!("Received Message Event"),
            InxEvent::Milestone => warn!("Received Milestone Event"),
        }
    }

    async fn run(&mut self) {
        while let Some(event) = self.receiver.recv().await {
            self.handle_event(event).await;
        }
    }
}

#[derive(Clone)]
struct BrokerAddr {
    sender: mpsc::UnboundedSender<InxEvent>,
}

impl InxListener {
    fn new(inx_client: InxClient<Channel>, broker_addr: BrokerAddr) -> Self {
        let message_handle = {
            let mut inx_client = inx_client.clone();
            let broker_addr = broker_addr.clone();
            tokio::spawn(async move { subscribe_messages(&mut inx_client, broker_addr).await })
        };

        let latest_milestone_handle = {
            let mut inx_client = inx_client.clone();
            let broker_addr = broker_addr.clone();
            tokio::spawn(async move { subscribe_latest_milestone(&mut inx_client, broker_addr).await })
        };

        Self {
            message_handle,
            latest_milestone_handle,
        }
    }

    fn shutdown(self) {
        self.message_handle.abort();
        self.latest_milestone_handle.abort();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let cli_args = CliArgs::parse();

    let (db, inx_client) = if let Some(config_path) = cli_args.config {
        let config = config::Config::from_file(config_path)?;
        (config.mongodb.build().await?, config.inx.build().await?)
    } else {
        (
            MongoConfig::new("mongodb://localhost:27017".into()).build().await?,
            InxClient::connect("http://localhost:9029").await?,
        )
    };

    let broker_addr = Broker::new(db);
    let inx_listener = InxListener::new(inx_client, broker_addr);

    tokio::signal::ctrl_c().await?;

    inx_listener.shutdown();

    Ok(())
}
