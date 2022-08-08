// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use dyn_clone::DynClone;

use super::{
    addr::{OptionalAddr, SendError},
    event::{DynEvent, Envelope},
    Actor,
};
use crate::runtime::{Addr, RuntimeError};

pub trait CloneSender<E>: Sender<E> + DynClone {}
impl<T, E> CloneSender<E> for T where T: Sender<E> + Clone {}
dyn_clone::clone_trait_object!(<E> CloneSender<E>);

/// Defines the sender half of a channel.
pub trait Sender<E>: IsClosed + Send + Sync {
    /// Sends a message over the channel.
    fn send(&self, event: E) -> Result<(), RuntimeError>;
}

/// Defines something that may be closed.
pub trait IsClosed {
    /// Returns whether the thing is closed.
    fn is_closed(&self) -> bool;
}

impl<E> Sender<E> for Box<dyn Sender<E>> {
    fn send(&self, event: E) -> Result<(), RuntimeError> {
        (**self).send(event)
    }
}

impl<E> IsClosed for Box<dyn Sender<E>> {
    fn is_closed(&self) -> bool {
        (**self).is_closed()
    }
}

impl<A> Sender<Envelope<A>> for tokio::sync::mpsc::UnboundedSender<Envelope<A>>
where
    A: Actor,
{
    fn send(&self, event: Envelope<A>) -> Result<(), RuntimeError> {
        self.send(event)
            .map_err(|_| RuntimeError::SendError("Failed to send event".into()))
    }
}

impl<E> IsClosed for tokio::sync::mpsc::UnboundedSender<E> {
    fn is_closed(&self) -> bool {
        self.is_closed()
    }
}

#[cfg(feature = "metrics-debug")]
impl<A> Sender<Envelope<A>> for bee_metrics::metrics::sync::mpsc::UnboundedSender<Envelope<A>>
where
    A: Actor,
{
    fn send(&self, event: Envelope<A>) -> Result<(), RuntimeError> {
        self.send(event)
            .map_err(|_| RuntimeError::SendError("Failed to send event".into()))
    }
}

#[cfg(feature = "metrics-debug")]
impl<E> IsClosed for bee_metrics::metrics::sync::mpsc::UnboundedSender<E> {
    fn is_closed(&self) -> bool {
        self.is_closed()
    }
}

impl<A, E> Sender<E> for Addr<A>
where
    A: Actor,
    E: 'static + DynEvent<A>,
{
    fn send(&self, event: E) -> Result<(), RuntimeError> {
        self.sender.send(Box::new(event))
    }
}

impl<A: Actor> IsClosed for Addr<A> {
    fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl<A, E> Sender<E> for OptionalAddr<A>
where
    A: Actor,
    E: 'static + DynEvent<A>,
{
    /// Sends an event if the address exists. Returns an error if the address is not set.
    fn send(&self, event: E) -> Result<(), RuntimeError> {
        self.0
            .as_ref()
            .ok_or_else(|| SendError::new(format!("No open address for {}", std::any::type_name::<A>())))?
            .send(event)
    }
}

impl<A: Actor> IsClosed for OptionalAddr<A> {
    fn is_closed(&self) -> bool {
        self.0.as_ref().map(|addr| addr.is_closed()).unwrap_or(true)
    }
}
