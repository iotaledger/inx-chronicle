// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use futures::Stream;

use super::{
    envelope::{Envelope, HandleEvent},
    handle::Addr,
    report::Report,
    Actor,
};
use crate::runtime::{config::SpawnConfig, scope::RuntimeScope, shutdown::ShutdownStream};

/// The context that an actor can use to interact with the runtime
pub struct ActorContext<A: Actor> {
    pub(crate) scope: RuntimeScope,
    pub(crate) handle: Addr<A>,
    pub(crate) receiver: ShutdownStream<Box<dyn Stream<Item = Envelope<A>> + Unpin + Send>>,
}

impl<A: Actor> ActorContext<A> {
    pub(crate) fn new(
        scope: RuntimeScope,
        handle: Addr<A>,
        receiver: ShutdownStream<Box<dyn Stream<Item = Envelope<A>> + Unpin + Send>>,
    ) -> Self {
        Self {
            handle,
            scope,
            receiver,
        }
    }

    /// Spawn a new supervised child actor
    pub async fn spawn_actor_supervised<OtherA, Cfg>(&mut self, actor: Cfg) -> Addr<OtherA>
    where
        OtherA: 'static + Actor + Debug + Send + Sync,
        A: 'static + Send + HandleEvent<Report<OtherA>>,
        Cfg: Into<SpawnConfig<OtherA>>,
    {
        let handle = self.handle().clone();
        self.scope.spawn_actor_supervised(actor, handle).await
    }

    /// Get this actors's handle
    pub fn handle(&self) -> &Addr<A> {
        &self.handle
    }

    /// Get the inbox
    pub fn inbox(&mut self) -> &mut ShutdownStream<Box<dyn Stream<Item = Envelope<A>> + Unpin + Send>> {
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
