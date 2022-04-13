// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::{self, MongoDatabase};
use log::debug;
use tokio::sync::mpsc;

use crate::listener::InxEvent;

#[cfg(feature = "stardust")]
use bee_message_stardust as stardust;

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
            InxEvent::Message {
                message: inx::Message { message, message_id },
                raw,
            } => {
                debug!("Received Message Event");
                self.db
                    .insert_one(db::model::stardust::Message {
                        message_id,
                        message,
                        raw,
                    }.to_bson::<>())
                    .await
                    .unwrap()
            }
            InxEvent::Milestone(inx::Milestone{milestone_id, milestone_index, message_id, milestone_timestamp, }) => {
                debug!("Received Milestone Event");
                self.db
                    .insert_one(db::model::stardust::Milestone{
                        milestone_id, milestone_index, milestone_timestamp, message_id
                    })
                    .await
                    .unwrap()
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
