// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, fmt::Debug, ops::Deref, panic::AssertUnwindSafe, sync::Arc};

use futures::{
    future::{AbortHandle, Abortable},
    Future, FutureExt,
};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::{
    actor::{context::ActorContext, envelope::HandleEvent, handle::Act, report::Report, Actor},
    error::RuntimeError,
    registry::{DepStatus, Scope, ScopeId, ROOT_SCOPE},
    shutdown::ShutdownHandle,
};
use crate::runtime::{
    actor::{envelope::Envelope, error::ActorError, report::ErrorReport},
    shutdown::ShutdownStream,
};

/// A view into a particular scope which provides the user-facing API
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
    /// Get the scope id
    pub fn id(&self) -> ScopeId {
        self.0.id
    }

    /// Get the parent scope, if one exists
    pub fn parent(&self) -> Option<ScopeView> {
        self.0.parent().cloned().map(ScopeView)
    }

    /// Get this scope's siblings
    pub async fn siblings(&self) -> Vec<ScopeView> {
        if let Some(parent) = self.0.parent() {
            parent.children().await.into_iter().map(ScopeView).collect()
        } else {
            vec![]
        }
    }

    /// Get this scope's children
    pub async fn children(&self) -> Vec<ScopeView> {
        self.0.children().await.into_iter().map(ScopeView).collect()
    }

    pub(crate) async fn add_data<T: 'static + Send + Sync + Clone>(&self, data: T) {
        log::debug!(
            "Adding {} to scope {:x}",
            std::any::type_name::<T>(),
            self.0.id.as_fields().0
        );
        self.0.add_data(std::any::TypeId::of::<T>(), Box::new(data)).await
    }

    pub(crate) async fn get_data_opt<T: 'static + Send + Sync + Clone>(&self) -> Option<T> {
        self.0
            .get_data(std::any::TypeId::of::<T>())
            .await
            .map(|res| res.with_type::<T>())
    }

    /// Query the registry for a dependency. This will return immediately whether or not it exists.
    pub async fn query_resource<T: 'static + Clone + Send + Sync>(&self) -> Result<T, RuntimeError> {
        self.get_data_opt()
            .await
            .ok_or_else(|| RuntimeError::MissingDependency(std::any::type_name::<Self>().to_string()))
    }

    /// Get the root scope
    pub fn root(&self) -> ScopeView {
        self.find_by_id(ROOT_SCOPE).unwrap()
    }

    /// Find a scope by id
    pub fn find_by_id(&self, scope_id: ScopeId) -> Option<ScopeView> {
        self.0.find(scope_id).cloned().map(ScopeView)
    }

    /// Shut down the scope
    pub async fn shutdown(&self) {
        self.0.shutdown().await;
    }

    /// Abort the tasks in this runtime's scope. This will shutdown tasks that have
    /// shutdown handles instead.
    pub(crate) async fn abort(&self) {
        self.0.abort().await;
    }

    /// Get an actor's event handle, if it exists in this scope.
    /// Note: This will only return a handle if the actor exists outside of a pool.
    pub async fn actor<A>(&self) -> Option<Act<A>>
    where
        A: 'static + Actor,
    {
        self.get_data_opt::<Act<A>>()
            .await
            .and_then(|handle| (!handle.is_closed()).then(|| handle))
    }

    /// Get a shared resource if it exists in this runtime's scope
    pub async fn resource<R: 'static + Send + Sync + Clone>(&self) -> Option<R> {
        self.get_data_opt::<R>().await
    }

    /// Add a shared resource
    pub async fn add_resource<R: 'static + Send + Sync + Clone>(&self, resource: R) {
        self.add_data(resource).await;
    }

    /// Add a global, shared resource and get a reference to it
    pub async fn add_global_resource<R: 'static + Send + Sync + Clone>(&self, resource: R) {
        self.root().add_data(resource).await;
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

    /// Create a new scope within this one
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

    /// Await the tasks in this runtime's scope
    pub(crate) async fn join(&mut self) {
        log::debug!("Joining scope {:x}", self.0.id.as_fields().0);
        for handle in self.join_handles.drain(..) {
            handle.await.ok();
        }
        self.0.drop().await;
    }

    pub(crate) async fn depend_on<T: 'static + Send + Sync + Clone>(&self) -> Result<DepStatus<T>, RuntimeError> {
        self.0
            .depend_on(std::any::TypeId::of::<T>())
            .await
            .map(|res| res.with_type::<T>())
    }

    pub(crate) async fn get_data<T: 'static + Send + Sync + Clone>(&self) -> Result<DepStatus<T>, RuntimeError> {
        self.0
            .get_data_promise(std::any::TypeId::of::<T>())
            .await
            .map(|res| res.with_type::<T>())
    }

    /// Request a dependency and wait for it to be available.
    pub async fn request_resource<T: 'static + Clone + Send + Sync>(&self) -> Result<T, RuntimeError> {
        self.get_data().await?.get().await
    }

    /// Request a dependency and wait for it to be added, forming a link between this scope and
    /// the requested data. If the data is removed from this scope, it will be shut down.
    pub async fn link_resource<T: 'static + Clone + Send + Sync>(&self) -> Result<T, RuntimeError> {
        log::debug!(
            "Linking resource {} in scope {:x}",
            std::any::type_name::<T>(),
            self.id().as_fields().0
        );
        self.depend_on().await?.get().await
    }

    /// Remove data from this scope
    pub async fn remove_data<T: 'static + Send + Sync + Clone>(&self) -> Option<T> {
        log::debug!(
            "Removing {} from scope {:x}",
            std::any::type_name::<T>(),
            self.0.id.as_fields().0
        );
        self.0
            .remove_data(std::any::TypeId::of::<T>())
            .await
            .map(|res| res.with_type::<T>())
    }

    /// Spawn a new, plain task
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
                                "{} exited with error: {:?}",
                                format!("Task {:x}", child_scope.id().as_fields().0),
                                e
                            );
                            Err(RuntimeError::ActorError(Arc::new(e)))
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

    /// Spawn a new actor with a supervisor handle
    pub async fn spawn_actor_supervised<A, Sup>(
        &mut self,
        mut actor: A,
        supervisor_handle: Act<Sup>,
    ) -> Result<Act<A>, RuntimeError>
    where
        A: 'static + Actor + Debug + Send + Sync,
        Sup: 'static + HandleEvent<Report<A>>,
    {
        if let Some(handle) = self.actor::<A>().await {
            if handle.scope_id() == self.id() {
                let name = actor.name();
                return Err(RuntimeError::DuplicateActor(
                    format!(
                        "Attempted to add a duplicate actor ({}) to scope {:x}",
                        name,
                        self.id().as_fields().0,
                    ),
                    self.id(),
                ));
            }
        }
        let (abort_handle, abort_reg) = AbortHandle::new_pair();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Envelope<A>>();
        let (receiver, shutdown_handle) = ShutdownStream::new(UnboundedReceiverStream::new(receiver));
        let scope = self.child(Some(shutdown_handle), Some(abort_handle)).await;
        let handle = Act::new(scope.scope.clone(), sender);
        let mut cx = ActorContext::new(scope, handle.clone(), receiver);
        log::debug!("Initializing {}", actor.name());
        self.add_data(handle.clone()).await;
        let res = AssertUnwindSafe(actor.init(&mut cx)).catch_unwind().await;
        let mut data = Self::handle_init_res::<A>(res, &mut cx).await?;
        let child_task = tokio::spawn(async move {
            let res = Abortable::new(
                AssertUnwindSafe(async {
                    // Call handle events until shutdown
                    let mut res = actor.start(&mut cx, &mut data).await;
                    if let Err(e) = actor.shutdown(&mut cx, &mut data).await {
                        res = Err(e);
                    }
                    res
                })
                .catch_unwind(),
                abort_reg,
            )
            .await;
            cx.scope.abort().await;
            cx.scope.join().await;
            match res {
                Ok(res) => match res {
                    Ok(res) => match res {
                        Ok(_) => {
                            supervisor_handle.send(Ok(actor))?;
                            Ok(())
                        }
                        Err(e) => {
                            log::error!("{} exited with error: {:?}", actor.name(), e);
                            let e = Arc::new(Box::new(e) as Box<dyn Error + Send + Sync>);
                            supervisor_handle.send(ErrorReport::new(actor, ActorError::Result(e.clone())))?;
                            Err(RuntimeError::ActorError(e))
                        }
                    },
                    Err(e) => {
                        supervisor_handle.send(ErrorReport::new(actor, ActorError::Panic))?;
                        std::panic::resume_unwind(e);
                    }
                },
                Err(_) => {
                    supervisor_handle.send(ErrorReport::new(actor, ActorError::Aborted))?;
                    Err(RuntimeError::AbortedScope(cx.scope.id()))
                }
            }
        });
        self.join_handles.push(child_task);
        Ok(handle)
    }

    /// Spawn a new actor with no supervisor
    pub async fn spawn_actor<A>(&mut self, mut actor: A) -> Result<Act<A>, RuntimeError>
    where
        A: 'static + Actor + Send + Sync,
    {
        if let Some(handle) = self.actor::<A>().await {
            if handle.scope_id() == self.id() {
                let name = actor.name();
                return Err(RuntimeError::DuplicateActor(
                    format!(
                        "Attempted to add a duplicate actor ({}) to scope {:x}",
                        name,
                        self.id().as_fields().0,
                    ),
                    self.id(),
                ));
            }
        }
        let (abort_handle, abort_reg) = AbortHandle::new_pair();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Envelope<A>>();
        let (receiver, shutdown_handle) = ShutdownStream::new(UnboundedReceiverStream::new(receiver));
        let scope = self.child(Some(shutdown_handle), Some(abort_handle)).await;
        let handle = Act::new(scope.scope.clone(), sender);
        let mut cx = ActorContext::new(scope, handle.clone(), receiver);
        log::debug!("Initializing {}", actor.name());
        self.add_data(handle.clone()).await;
        let res = AssertUnwindSafe(actor.init(&mut cx)).catch_unwind().await;
        let mut data = Self::handle_init_res::<A>(res, &mut cx).await?;
        let child_task = tokio::spawn(async move {
            let res = Abortable::new(
                AssertUnwindSafe(async {
                    // Call handle events until shutdown
                    let mut res = actor.start(&mut cx, &mut data).await;
                    if let Err(e) = actor.shutdown(&mut cx, &mut data).await {
                        res = Err(e);
                    }
                    res
                })
                .catch_unwind(),
                abort_reg,
            )
            .await;
            cx.scope.abort().await;
            cx.scope.join().await;
            match res {
                Ok(res) => match res {
                    Ok(res) => match res {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            log::error!("{} exited with error: {:?}", actor.name(), e);
                            Err(RuntimeError::ActorError(Arc::new(
                                Box::new(e) as Box<dyn Error + Send + Sync>
                            )))
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
        Ok(handle)
    }

    pub(crate) async fn handle_init_res<A>(
        res: std::thread::Result<Result<A::Data, A::Error>>,
        cx: &mut ActorContext<A>,
    ) -> Result<A::Data, RuntimeError>
    where
        A: 'static + Actor,
    {
        match res {
            Ok(res) => match res {
                Ok(d) => Ok(d),
                Err(e) => {
                    cx.abort().await;
                    cx.join().await;
                    return Err(RuntimeError::ActorError(Arc::new(
                        Box::new(e) as Box<dyn Error + Send + Sync>
                    )));
                }
            },
            Err(e) => {
                cx.abort().await;
                cx.join().await;
                std::panic::resume_unwind(e);
            }
        }
    }
}
