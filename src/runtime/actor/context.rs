// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use tokio_stream::wrappers::UnboundedReceiverStream;

use super::{
    envelope::{Envelope, HandleEvent},
    handle::Act,
    report::Report,
    Actor,
};
use crate::runtime::{error::RuntimeError, scope::RuntimeScope, shutdown::ShutdownStream};

pub struct ActorContext<A: Actor> {
    pub(crate) scope: RuntimeScope,
    pub(crate) handle: Act<A>,
    pub(crate) receiver: ShutdownStream<UnboundedReceiverStream<Envelope<A>>>,
}

impl<A: Actor> ActorContext<A> {
    pub(crate) fn new(
        scope: RuntimeScope,
        handle: Act<A>,
        receiver: ShutdownStream<UnboundedReceiverStream<Envelope<A>>>,
    ) -> Self {
        Self {
            handle,
            scope,
            receiver,
        }
    }

    /// Spawn a new supervised child actor
    pub async fn spawn_actor_supervised<OtherA>(&mut self, actor: OtherA) -> Result<Act<OtherA>, RuntimeError>
    where
        OtherA: 'static + Actor + Debug + Send + Sync,
        A: 'static + Send + HandleEvent<Report<OtherA>>,
    {
        let handle = self.handle().clone();
        self.scope.spawn_actor_supervised(actor, handle).await
    }

    /// Get this actors's handle
    pub fn handle(&self) -> &Act<A> {
        &self.handle
    }

    /// Get the inbox
    pub fn inbox(&mut self) -> &mut ShutdownStream<UnboundedReceiverStream<Envelope<A>>> {
        &mut self.receiver
    }

    /// Shutdown the actor
    pub async fn shutdown(&self) {
        self.handle().shutdown().await;
    }
}

impl<A: Actor> Deref for ActorContext<A> {
    type Target = RuntimeScope;

    fn deref(&self) -> &Self::Target {
        &self.scope
    }
}

impl<A: Actor> DerefMut for ActorContext<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.scope
    }
}
