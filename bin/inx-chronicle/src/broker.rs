// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::MongoDatabase;
use log::debug;
use tokio::sync::mpsc;

use crate::listener::InxEvent;

pub struct Broker {
    db: MongoDatabase,
    receiver: mpsc::UnboundedReceiver<InxEvent>,
}

impl Broker {
    pub fn register(db: MongoDatabase) -> BrokerAddr {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut broker = Broker { receiver, db };
        tokio::spawn(async move { broker.run().await });
        BrokerAddr { sender }
    }

    pub(crate) async fn handle_event(&mut self, event: InxEvent) {
        match event {
            InxEvent::Message(message) => {
                debug!("Received Message Event");
                self.db.insert_message_raw(message).await.unwrap()
            }
            InxEvent::LatestMilestone(milestone) => {
                debug!("Received Milestone Event");
                self.db.insert_milestone(milestone).await.unwrap()
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        while let Some(event) = self.receiver.recv().await {
            self.handle_event(event).await;
        }
    }
}

#[derive(Clone)]
pub struct BrokerAddr {
    sender: mpsc::UnboundedSender<InxEvent>,
}

impl BrokerAddr {
    pub fn send(&self, event: InxEvent) {
        self.sender.send(event).unwrap()
    }
}
