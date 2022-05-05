// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    any::Any,
    ops::{Deref, DerefMut},
    panic::AssertUnwindSafe,
    time::Duration,
};

use futures::{
    future::{AbortRegistration, Abortable, Aborted},
    FutureExt,
};

use super::{
    addr::Addr,
    event::{DynEvent, EnvelopeStream, HandleEvent},
    report::Report,
    util::DelayedEvent,
    Actor,
};
use crate::runtime::{config::SpawnConfig, error::RuntimeError, scope::RuntimeScope, shutdown::ShutdownStream};

type Receiver<A> = ShutdownStream<EnvelopeStream<A>>;

/// The context that an actor can use to interact with the runtime.
pub struct ActorContext<A: Actor> {
    pub(crate) scope: RuntimeScope,
    pub(crate) handle: Addr<A>,
    pub(crate) receiver: Receiver<A>,
}

impl<A: Actor> ActorContext<A> {
    pub(crate) fn new(scope: RuntimeScope, handle: Addr<A>, receiver: Receiver<A>) -> Self {
        Self {
            handle,
            scope,
            receiver,
        }
    }

    /// Spawn a new supervised child actor.
    pub async fn spawn_child<OtherA, Cfg>(&mut self, actor: Cfg) -> Addr<OtherA>
    where
        OtherA: 'static + Actor,
        A: 'static + HandleEvent<Report<OtherA>>,
        Cfg: Into<SpawnConfig<OtherA>>,
    {
        let handle = self.handle().clone();
        self.scope.spawn_actor(actor, handle).await
    }

    /// Get this actor's handle.
    pub fn handle(&self) -> &Addr<A> {
        &self.handle
    }

    /// Get the inbox.
    pub fn inbox(&mut self) -> &mut Receiver<A> {
        &mut self.receiver
    }

    /// Delay the processing of an event by re-sending it to self.
    /// If a time is specified, the event will be delayed until that time,
    /// otherwise it will re-process immediately.
    pub fn delay<E: 'static + DynEvent<A>>(
        &self,
        event: E,
        delay: impl Into<Option<Duration>>,
    ) -> Result<(), RuntimeError>
    where
        A: 'static,
    {
        match delay.into() {
            Some(delay) => self.handle.send(DelayedEvent::new(event, delay)),
            None => self.handle.send(event),
        }
    }

    /// Shutdown the actor.
    pub fn shutdown(&self) {
        self.handle().shutdown();
    }

    pub(crate) async fn start(
        &mut self,
        actor: &mut A,
        actor_state: &mut Option<A::State>,
        abort_reg: AbortRegistration,
    ) -> Result<Result<Result<(), A::Error>, Box<dyn Any + Send>>, Aborted> {
        let res = Abortable::new(
            AssertUnwindSafe(async {
                let mut state = actor.init(self).await?;
                // Call handle events until shutdown
                let mut res = actor.run(self, &mut state).await;
                if let Err(e) = actor.shutdown(self, &mut state).await {
                    res = Err(e);
                }
                actor_state.replace(state);
                res
            })
            .catch_unwind(),
            abort_reg,
        )
        .await;
        self.scope.abort().await;
        self.scope.join().await;
        res
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
