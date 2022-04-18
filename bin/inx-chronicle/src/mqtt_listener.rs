// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`MqttListener`] subscribes to events from INX and forwards them via a Tokio unbounded
//! channel.

use std::{fmt::Debug, io::Cursor, marker::PhantomData};

use async_trait::async_trait;
use bee_common_chrysalis::packable::Packable;
use bee_message_chrysalis::Message;
use chronicle::{
    db::model::{
        chrysalis::{message::MessageRecord, milestone::MilestoneRecord},
        ConversionError,
    },
    mqtt::{MqttConfig, MqttError},
    runtime::{
        actor::{addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, Actor},
        config::ConfigureActor,
        error::RuntimeError,
    },
};
use futures::Stream;
use rumqttc::{ClientError, ConnectionError, Event, EventLoop, Incoming, Publish};
use thiserror::Error;

use crate::broker::Broker;

type MessageStream = MqttStreamListener<MessageRecord>;
type MilestoneStream = MqttStreamListener<MilestoneRecord>;

const MESSAGE_TOPIC: &str = "messages";
const MESSAGE_CLIENT_ID: &str = "chronicle-messages";
const MILESTONE_TOPIC: &str = "milestones/latest";
const MILESTONE_CLIENT_ID: &str = "chronicle-milestones";

fn mqtt_stream(mut event_loop: EventLoop) -> impl Stream<Item = Result<Event, ConnectionError>> + Unpin {
    Box::pin(async_stream::stream! {
        loop {
            let event = event_loop.poll().await;
            if let Err(e) = &event {
                match e {
                    ConnectionError::StreamDone | ConnectionError::Cancel => break,
                    _ => yield event,
                }
            } else {
                yield event;
            }
        }
    })
}

#[derive(Debug, Error)]
pub enum MqttListenerError {
    #[error("The broker actor is not running")]
    MissingBroker,
    #[error(transparent)]
    MqttOptions(#[from] MqttError),
    #[error(transparent)]
    MqttClient(#[from] ClientError),
    #[error(transparent)]
    MqttConnection(#[from] ConnectionError),
    #[error(transparent)]
    Unpack(#[from] bee_message_chrysalis::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Conversion(#[from] ConversionError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Debug)]
pub struct MqttListener {
    config: MqttConfig,
    broker_addr: Addr<Broker>,
}

impl MqttListener {
    pub fn new(config: MqttConfig, broker_addr: Addr<Broker>) -> Self {
        Self { config, broker_addr }
    }
}

#[async_trait]
impl Actor for MqttListener {
    type State = ();
    type Error = MqttListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let (messages_client, message_loop) = self.config.build(MESSAGE_CLIENT_ID)?;
        messages_client
            .subscribe(MESSAGE_TOPIC, rumqttc::QoS::AtLeastOnce)
            .await?;

        cx.spawn_actor_supervised::<MessageStream, _>(
            MqttStreamListener::new(self.broker_addr.clone())?.with_stream(mqtt_stream(message_loop)),
        )
        .await;

        let (milestones_client, milestone_loop) = self.config.build(MILESTONE_CLIENT_ID)?;
        milestones_client
            .subscribe(MILESTONE_TOPIC, rumqttc::QoS::AtLeastOnce)
            .await?;

        cx.spawn_actor_supervised::<MilestoneStream, _>(
            MqttStreamListener::new(self.broker_addr.clone())?.with_stream(mqtt_stream(milestone_loop)),
        )
        .await;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<MessageStream>> for MqttListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MessageStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match e.error {
                ActorError::Result(_) => {
                    let (messages_client, message_loop) = self.config.build(MESSAGE_CLIENT_ID)?;
                    messages_client
                        .subscribe(MESSAGE_TOPIC, rumqttc::QoS::AtLeastOnce)
                        .await?;

                    cx.spawn_actor_supervised::<MessageStream, _>(
                        MqttStreamListener::new(self.broker_addr.clone())?.with_stream(mqtt_stream(message_loop)),
                    )
                    .await;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<MilestoneStream>> for MqttListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MilestoneStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match e.error {
                ActorError::Result(_) => {
                    let (milestones_client, milestone_loop) = self.config.build(MILESTONE_CLIENT_ID)?;
                    milestones_client
                        .subscribe(MILESTONE_TOPIC, rumqttc::QoS::AtLeastOnce)
                        .await?;

                    cx.spawn_actor_supervised::<MilestoneStream, _>(
                        MqttStreamListener::new(self.broker_addr.clone())?.with_stream(mqtt_stream(milestone_loop)),
                    )
                    .await;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

pub struct MqttStreamListener<I> {
    broker_addr: Addr<Broker>,
    _item: PhantomData<I>,
}

impl<I> MqttStreamListener<I> {
    pub fn new(broker_addr: Addr<Broker>) -> Result<Self, MqttListenerError> {
        if broker_addr.is_closed() {
            Err(MqttListenerError::MissingBroker)
        } else {
            Ok(Self {
                broker_addr,
                _item: PhantomData,
            })
        }
    }
}

#[async_trait]
impl Actor for MqttStreamListener<MessageRecord> {
    type State = ();
    type Error = MqttListenerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Result<Event, ConnectionError>> for MqttStreamListener<MessageRecord> {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        event: Result<Event, ConnectionError>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let event = event?;
        if let Event::Incoming(Incoming::Publish(Publish { payload, .. })) = event {
            let message = Message::unpack(&mut Cursor::new(&payload))?;
            self.broker_addr
                .send(MessageRecord::new(message.id().0, message, payload.as_ref().into()))
                .map_err(RuntimeError::SendError)?;
        } else {
            log::trace!("MqttStreamListener<MessageRecord> received {:?}", event);
        }
        Ok(())
    }
}

#[async_trait]
impl Actor for MqttStreamListener<MilestoneRecord> {
    type State = ();
    type Error = MqttListenerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Result<Event, ConnectionError>> for MqttStreamListener<MilestoneRecord> {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        event: Result<Event, ConnectionError>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let event = event?;
        if let Event::Incoming(Incoming::Publish(Publish { payload, .. })) = event {
            let rec = MilestoneRecord::try_from(serde_json::from_slice::<serde_json::Value>(&payload)?)?;
            self.broker_addr.send(rec).map_err(RuntimeError::SendError)?;
        } else {
            log::trace!("MqttStreamListener<MilestoneRecord> received {:?}", event);
        }
        Ok(())
    }
}

impl<I> Debug for MqttStreamListener<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttStreamListener")
            .field("item", &std::any::type_name::<I>())
            .finish()
    }
}
