// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, fmt::Debug, ops::Deref, panic::AssertUnwindSafe};

use futures::{
    future::{AbortHandle, AbortRegistration, Abortable},
    Future, FutureExt,
};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace, warn};

use super::{
    actor::{
        addr::{Addr, OptionalAddr},
        context::ActorContext,
        error::ActorError,
        event::{Envelope, HandleEvent},
        report::{ErrorReport, Report, SuccessReport},
        Actor,
    },
    config::{SpawnConfig, SpawnConfigInner},
    error::RuntimeError,
    registry::{Scope, ScopeId, ROOT_SCOPE},
    shutdown::{ShutdownHandle, ShutdownStream},
    spawn_task,
    task::{
        error::TaskError,
        report::{TaskErrorReport, TaskReport, TaskSuccessReport},
        Task,
    },
    Sender,
};
use crate::runtime::{error::ErrorLevel, merge::Merge};

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

    /// Searches for an address for the given actor type
    /// and returns an optional result.
    pub async fn addr<A: 'static + Actor>(&self) -> OptionalAddr<A> {
        self.0.get_addr().await
    }

    /// Shuts down the scope.
    pub async fn shutdown(&self) {
        self.0.shutdown().await;
    }

    /// Aborts the tasks in this runtime's scope. This will shutdown tasks that have
    /// shutdown handles instead.
    pub async fn abort(&self) {
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

    pub(crate) async fn child(&self, abort_handle: AbortHandle) -> Self {
        Self {
            scope: ScopeView(self.scope.0.child(abort_handle).await),
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
        let mut child_scope = self.child(abort_handle).await;
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
        debug!("Joining scope {:x}", self.0.id.as_fields().0);
        for handle in self.join_handles.drain(..) {
            handle.await.ok();
        }
        self.0.drop().await;
    }

    async fn common_spawn<A>(
        &mut self,
        actor: &A,
        SpawnConfigInner {
            streams,
            add_to_registry,
        }: SpawnConfigInner<A>,
    ) -> (Addr<A>, ActorContext<A>, AbortRegistration, ShutdownHandle)
    where
        A: 'static + Actor,
    {
        let (abort_handle, abort_reg) = AbortHandle::new_pair();
        let (sender, receiver) = {
            let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Envelope<A>>();
            (sender, tokio_stream::wrappers::UnboundedReceiverStream::new(receiver))
        };

        let (receiver, shutdown_handle) = ShutdownStream::new(Box::new(receiver));
        let receiver = match streams {
            Some(mut streams) => {
                streams.merge(receiver);
                streams
            }
            None => Merge::new(receiver),
        };
        let scope = self.child(abort_handle).await;
        let handle = Addr::new(scope.scope.clone(), sender);
        if add_to_registry {
            self.scope.0.insert_addr(handle.clone()).await;
        }
        let cx = ActorContext::new(scope, handle.clone(), receiver);
        debug!("Initializing {}", actor.name());
        (handle, cx, abort_reg, shutdown_handle)
    }

    /// Spawns a new, plain task.
    pub async fn spawn_task<T, Sup>(&mut self, mut task: T, supervisor_addr: Addr<Sup>) -> AbortHandle
    where
        T: Task + 'static,
        Sup: 'static + HandleEvent<TaskReport<T>>,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let mut child_scope = self.child(abort_handle.clone()).await;
        let child_task = spawn_task(task.name().as_ref(), async move {
            let fut = task.run();
            let res = Abortable::new(AssertUnwindSafe(fut).catch_unwind(), abort_registration).await;
            child_scope.join().await;
            match res {
                Ok(res) => match res {
                    Ok(res) => match res {
                        Ok(_) => {
                            supervisor_addr.send(TaskReport::Success(TaskSuccessReport::new(task)))?;
                            Ok(())
                        }
                        Err(e) => {
                            handle_dynamic_error(format!("Task {:x}", child_scope.id().as_fields().0), &e);
                            Err(RuntimeError::ActorError(e.to_string()))
                        }
                    },
                    Err(e) => {
                        supervisor_addr.send(TaskReport::Error(TaskErrorReport::new(task, TaskError::Panic)))?;
                        std::panic::resume_unwind(e);
                    }
                },
                Err(_) => {
                    supervisor_addr.send(TaskReport::Error(TaskErrorReport::new(task, TaskError::Aborted)))?;
                    Err(RuntimeError::AbortedScope(child_scope.id()))
                }
            }
        });
        self.join_handles.push(child_task);
        abort_handle
    }

    /// Spawns a new actor with a supervisor handle.
    pub async fn spawn_actor<A, Cfg, Sup>(&mut self, actor: Cfg, supervisor_addr: Addr<Sup>) -> Addr<A>
    where
        A: 'static + Actor,
        Sup: 'static + HandleEvent<Report<A>>,
        Cfg: Into<SpawnConfig<A>>,
    {
        let SpawnConfig { mut actor, config } = actor.into();
        let (handle, mut cx, abort_reg, shutdown_handle) = self.common_spawn(&actor, config).await;
        let child_task = spawn_task(actor.name().as_ref(), async move {
            let mut data = None;
            let res = cx.start(&mut actor, &mut data, abort_reg, shutdown_handle).await;
            match res {
                Ok(res) => match res {
                    Ok(res) => match res {
                        Ok(_) => {
                            supervisor_addr.send(Report::Success(SuccessReport::new(actor, data)))?;
                            Ok(())
                        }
                        Err(e) => {
                            handle_dynamic_error(actor.name(), &e);
                            let err_str = e.to_string();
                            supervisor_addr.send(Report::Error(ErrorReport::new(
                                actor,
                                data,
                                ActorError::Result(e),
                            )))?;
                            Err(RuntimeError::ActorError(err_str))
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
    pub async fn spawn_actor_unsupervised<A, Cfg>(&mut self, actor: Cfg) -> Addr<A>
    where
        A: 'static + Actor,
        Cfg: Into<SpawnConfig<A>>,
    {
        let SpawnConfig { mut actor, config } = actor.into();
        let (handle, mut cx, abort_reg, shutdown_handle) = self.common_spawn(&actor, config).await;
        let child_task = spawn_task(actor.name().as_ref(), async move {
            let mut data = None;
            let res = cx.start(&mut actor, &mut data, abort_reg, shutdown_handle).await;
            match res {
                Ok(res) => match res {
                    Ok(res) => match res {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            handle_dynamic_error(actor.name(), &e);
                            Err(RuntimeError::ActorError(e.to_string()))
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

fn handle_dynamic_error(name: impl std::fmt::Display, e: &impl ErrorLevel) {
    use tracing::Level;
    match e.level() {
        Level::INFO => {
            info!(
                "{} exited with error: {} (but we all agreed to look the other way)",
                name, e
            );
        }
        Level::WARN => {
            warn!("{} exited with a warning: {}", name, e);
        }
        Level::ERROR => {
            error!("{} exited with error: {}", name, e);
        }
        Level::DEBUG => {
            debug!("{} exited with error: {}", name, e);
        }
        Level::TRACE => {
            trace!("{} exited with error: {}", name, e);
        }
    }
}
