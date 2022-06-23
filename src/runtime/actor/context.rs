// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    any::Any,
    ops::{Deref, DerefMut},
    panic::AssertUnwindSafe,
    time::Duration,
};

use futures::{
    future::{AbortHandle, AbortRegistration, Abortable, Aborted},
    FutureExt, Stream, StreamExt,
};

use super::{
    addr::Addr,
    event::{DynEvent, Envelope, HandleEvent},
    report::Report,
    util::DelayedEvent,
    Actor,
};
use crate::runtime::{
    config::SpawnConfig, error::RuntimeError, merge::Merge, scope::RuntimeScope, shutdown::ShutdownHandle, Sender,
    Task, TaskReport,
};

type Receiver<A> = Merge<Envelope<A>>;

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

    /// Spawn a new supervised child task.
    pub async fn spawn_child_task<T>(&mut self, task: T) -> AbortHandle
    where
        T: 'static + Task,
        A: 'static + HandleEvent<TaskReport<T>>,
    {
        let handle = self.handle().clone();
        self.scope.spawn_task(task, handle).await
    }

    /// Get this actor's handle.
    pub fn handle(&self) -> &Addr<A> {
        &self.handle
    }

    /// Get the inbox.
    pub fn inbox(&mut self) -> &mut (impl Stream<Item = Envelope<A>> + Send) {
        &mut self.receiver
    }

    /// Add an additional stream of events to the inbox.
    pub fn add_stream<S>(&mut self, stream: S)
    where
        S: 'static + Stream + Unpin + Send,
        S::Item: 'static + DynEvent<A>,
    {
        self.receiver
            .merge(Box::new(stream.map(|e| Box::new(e) as Envelope<A>)));
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

    pub(crate) async fn start(
        &mut self,
        actor: &mut A,
        actor_state: &mut Option<A::State>,
        abort_reg: AbortRegistration,
        shutdown_handle: ShutdownHandle,
    ) -> Result<Result<Result<(), A::Error>, Box<dyn Any + Send>>, Aborted> {
        let res = Abortable::new(
            AssertUnwindSafe(async {
                let mut state = actor.init(self).await?;
                // Set the shutdown handle before starting the event loop.
                self.scope.0.set_shutdown_handle(shutdown_handle).await;
                // Call handle events until shutdown
                let res = actor.run(self, &mut state).await;
                let res = actor.shutdown(self, &mut state, res).await;
                actor_state.replace(state);
                res
            })
            .catch_unwind(),
            abort_reg,
        )
        .await;
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
