// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxListener`] subscribes to events from INX and forwards them as an [`InxEvent`] via a Tokio unbounded
//! channel.

use futures::StreamExt;
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    Channel,
};
use log::info;
use tokio::task::JoinHandle;

use crate::broker::BrokerAddr;

#[derive(Debug)]
pub enum InxEvent {
    Message(inx::proto::Message),
    LatestMilestone(inx::proto::Milestone),
}

pub struct InxListener {
    // TODO: Consider storing `inx_client` to potentially restart some streams.
    message_handle: JoinHandle<()>,
    latest_milestone_handle: JoinHandle<()>,
}

impl InxListener {
    pub fn new(inx_client: InxClient<Channel>, broker_addr: BrokerAddr) -> Self {
        let message_handle = {
            let mut inx_client = inx_client.clone();
            let broker_addr = broker_addr.clone();
            tokio::spawn(async move { subscribe_messages(&mut inx_client, broker_addr).await })
        };

        let latest_milestone_handle = {
            let mut inx_client = inx_client;
            tokio::spawn(async move { subscribe_latest_milestone(&mut inx_client, broker_addr).await })
        };

        Self {
            message_handle,
            latest_milestone_handle,
        }
    }

    pub fn shutdown(self) {
        self.message_handle.abort();
        self.latest_milestone_handle.abort();
    }
}

async fn subscribe_messages(client: &mut InxClient<Channel>, broker_addr: BrokerAddr) {
    let response = client.listen_to_messages(MessageFilter {}).await;
    info!("Subscribed to `ListenToMessages`.");
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        let message = item.unwrap();
        broker_addr.send(InxEvent::Message(message));
    }
}

async fn subscribe_latest_milestone(client: &mut InxClient<Channel>, broker_addr: BrokerAddr) {
    let response = client.listen_to_latest_milestone(NoParams {}).await;
    info!("Subscribed to `ListenToLatestMilestone`.");
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        let milestone = item.unwrap();
        broker_addr.send(InxEvent::LatestMilestone(milestone));
    }
}
