// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, fmt::Debug, ops::Deref, panic::AssertUnwindSafe, sync::Arc};

use futures::{
    future::{AbortHandle, AbortRegistration, Abortable},
    Future, FutureExt,
};
use tokio::task::JoinHandle;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

use super::{
    actor::{
        addr::Addr,
        context::ActorContext,
        event::{EnvelopeStream, HandleEvent},
        report::{Report, SuccessReport},
        Actor,
    },
    config::SpawnConfig,
    error::RuntimeError,
    registry::{Scope, ScopeId, ROOT_SCOPE},
    shutdown::ShutdownHandle,
};
use crate::runtime::{
    actor::{error::ActorError, event::Envelope, report::ErrorReport},
    shutdown::ShutdownStream,
};

/// A view into a particular scope which provides the user-facing API.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ScopeView(pub(crate) Scope);

impl Deref for RuntimeScope {
    type Target = ScopeView;

    fn deref(&self) -> &Self::Target {
        &self.scope
    }
}

impl ScopeView {
    /// Gets the scope id.
    pub fn id(&self) -> ScopeId {
        self.0.id
    }

    /// Gets the parent scope, if one exists.
    pub fn parent(&self) -> Option<ScopeView> {
        self.0.parent().cloned().map(ScopeView)
    }

    /// Gets this scope's siblings.
    pub async fn siblings(&self) -> Vec<ScopeView> {
        if let Some(parent) = self.0.parent() {
            parent.children().await.into_iter().map(ScopeView).collect()
        } else {
            vec![]
        }
    }

    /// Gets this scope's children.
    pub async fn children(&self) -> Vec<ScopeView> {
        self.0.children().await.into_iter().map(ScopeView).collect()
    }

    /// Gets the root scope.
    pub fn root(&self) -> ScopeView {
        // Unwrap: the root scope is guaranteed to exist
        self.find_by_id(ROOT_SCOPE).unwrap()
    }

    /// Finds a scope by id.
    pub fn find_by_id(&self, scope_id: ScopeId) -> Option<ScopeView> {
        self.0.find(scope_id).cloned().map(ScopeView)
    }

    /// Shuts down the scope.
    pub fn shutdown(&self) {
        self.0.shutdown();
    }

    /// Aborts the tasks in this runtime's scope. This will shutdown tasks that have
    /// shutdown handles instead.
    pub(crate) async fn abort(&self) {
        self.0.abort().await;
    }
}

/// A runtime which defines a particular scope and functionality to
/// create tasks within it.
#[derive(Debug)]
pub struct RuntimeScope {
    pub(crate) scope: ScopeView,
    pub(crate) join_handles: Vec<JoinHandle<Result<(), RuntimeError>>>,
}

impl RuntimeScope {
    pub(crate) fn root(abort_handle: AbortHandle) -> Self {
        let scope = ScopeView(Scope::root(abort_handle));
        Self {
            scope,
            join_handles: Default::default(),
        }
    }

    pub(crate) async fn child(
        &self,
        shutdown_handle: Option<ShutdownHandle>,
        abort_handle: Option<AbortHandle>,
    ) -> Self {
        Self {
            scope: ScopeView(self.scope.0.child(shutdown_handle, abort_handle).await),
            join_handles: Default::default(),
        }
    }

    /// Creates a new scope within this one.
    pub async fn scope<S, F, O>(&self, f: S) -> Result<O, RuntimeError>
    where
        O: Send + Sync,
        S: Send + FnOnce(&mut RuntimeScope) -> F,
        F: Future<Output = Result<O, Box<dyn Error + Send + Sync>>>,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let mut child_scope = self.child(None, Some(abort_handle)).await;
        let res = Abortable::new(f(&mut child_scope), abort_registration).await;
        if let Ok(Err(_)) = res {
            child_scope.abort().await;
        }
        child_scope.join().await;
        match res {
            Ok(res) => res.map_err(|e| RuntimeError::ScopeLaunchError(e)),
            Err(_) => Err(RuntimeError::AbortedScope(child_scope.id())),
        }
    }

    /// Awaits the tasks in this runtime's scope.
    pub(crate) async fn join(&mut self) {
        log::debug!("Joining scope {:x}", self.0.id.as_fields().0);
        for handle in self.join_handles.drain(..) {
            handle.await.ok();
        }
        self.0.drop().await;
    }

    /// Spawns a new, plain task.
    pub async fn spawn_task<T, F>(&mut self, f: T) -> AbortHandle
    where
        T: Send + FnOnce(&mut RuntimeScope) -> F,
        F: 'static + Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let mut child_scope = self.child(None, Some(abort_handle.clone())).await;
        let fut = f(&mut child_scope);
        let child_task = tokio::spawn(async move {
            let res = Abortable::new(AssertUnwindSafe(fut).catch_unwind(), abort_registration).await;
            child_scope.abort().await;
            child_scope.join().await;
            match res {
                Ok(res) => match res {
                    Ok(res) => match res {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            log::error!(
                                "{} exited with error: {}",
                                format!("Task {:x}", child_scope.id().as_fields().0),
                                e
                            );
                            Err(RuntimeError::TaskError(e))
                        }
                    },
                    Err(e) => {
                        std::panic::resume_unwind(e);
                    }
                },
                Err(_) => Err(RuntimeError::AbortedScope(child_scope.id())),
            }
        });
        self.join_handles.push(child_task);
        abort_handle
    }

    async fn common_spawn<A>(
        &mut self,
        actor: &A,
        stream: Option<EnvelopeStream<A>>,
    ) -> (Addr<A>, ActorContext<A>, AbortRegistration)
    where
        A: 'static + Actor + Send + Sync,
    {
        let (abort_handle, abort_reg) = AbortHandle::new_pair();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Envelope<A>>();
        let receiver = UnboundedReceiverStream::new(receiver);
        let (receiver, shutdown_handle) = if let Some(stream) = stream {
            let receiver = receiver.merge(stream);
            let (receiver, shutdown_handle) = ShutdownStream::new(Box::new(receiver) as _);
            (receiver, shutdown_handle)
        } else {
            let (receiver, shutdown_handle) = ShutdownStream::new(Box::new(receiver) as _);
            (receiver, shutdown_handle)
        };
        let scope = self.child(Some(shutdown_handle), Some(abort_handle)).await;
        let handle = Addr::new(scope.scope.clone(), sender);
        let cx = ActorContext::new(scope, handle.clone(), receiver);
        log::debug!("Initializing {}", actor.name());
        (handle, cx, abort_reg)
    }

    /// Spawns a new actor with a supervisor handle.
    pub async fn spawn_actor_supervised<A, Cfg, Sup>(&mut self, actor: Cfg, supervisor_addr: Addr<Sup>) -> Addr<A>
    where
        A: 'static + Actor + Debug + Send + Sync,
        Sup: 'static + HandleEvent<Report<A>>,
        Cfg: Into<SpawnConfig<A>>,
    {
        let SpawnConfig { mut actor, stream } = actor.into();
        let (handle, mut cx, abort_reg) = self.common_spawn(&actor, stream).await;
        let child_task = tokio::spawn(async move {
            let mut data = None;
            let res = cx.start(&mut actor, &mut data, abort_reg).await;
            match res {
                Ok(res) => match res {
                    Ok(res) => match res {
                        Ok(_) => {
                            supervisor_addr.send(Report::Success(SuccessReport::new(actor, data)))?;
                            Ok(())
                        }
                        Err(e) => {
                            log::error!("{} exited with error: {}", actor.name(), e);
                            let e = Arc::new(e);
                            supervisor_addr.send(Report::Error(ErrorReport::new(
                                actor,
                                data,
                                ActorError::Result(e.clone()),
                            )))?;
                            Err(RuntimeError::ActorError(e))
                        }
                    },
                    Err(e) => {
                        supervisor_addr.send(Report::Error(ErrorReport::new(actor, data, ActorError::Panic)))?;
                        std::panic::resume_unwind(e);
                    }
                },
                Err(_) => {
                    supervisor_addr.send(Report::Error(ErrorReport::new(actor, data, ActorError::Aborted)))?;
                    Err(RuntimeError::AbortedScope(cx.scope.id()))
                }
            }
        });
        self.join_handles.push(child_task);
        handle
    }

    /// Spawns a new actor with no supervisor.
    pub async fn spawn_actor<A, Cfg>(&mut self, actor: Cfg) -> Addr<A>
    where
        A: 'static + Actor + Send + Sync,
        Cfg: Into<SpawnConfig<A>>,
    {
        let SpawnConfig { mut actor, stream } = actor.into();
        let (handle, mut cx, abort_reg) = self.common_spawn(&actor, stream).await;
        let child_task = tokio::spawn(async move {
            let mut data = None;
            let res = cx.start(&mut actor, &mut data, abort_reg).await;
            match res {
                Ok(res) => match res {
                    Ok(res) => match res {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            log::error!("{} exited with error: {}", actor.name(), e);
                            Err(RuntimeError::ActorError(Arc::new(e)))
                        }
                    },
                    Err(e) => {
                        std::panic::resume_unwind(e);
                    }
                },
                Err(_) => Err(RuntimeError::AbortedScope(cx.scope.id())),
            }
        });
        self.join_handles.push(child_task);
        handle
    }
}
