// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    any::TypeId,
    collections::{hash_map::Entry, HashMap},
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use anymap::{CloneAny, Downcast};
use async_recursion::async_recursion;
use futures::{future::AbortHandle, task::AtomicWaker, Future};
use tokio::sync::RwLock;
pub use uuid::Uuid;

use super::{error::RuntimeError, shutdown::ShutdownHandle};

/// An alias type indicating that this is a scope id
pub type ScopeId = Uuid;

/// The root scope id, which is always a zeroed uuid
pub const ROOT_SCOPE: Uuid = Uuid::nil();

/// A scope, which marks data as usable for a given task
#[derive(Clone, Debug)]
pub struct Scope {
    pub(crate) inner: Arc<ScopeInner>,
    valid: Arc<AtomicBool>,
}

/// Shared scope information
#[derive(Debug)]
pub struct ScopeInner {
    pub(crate) id: ScopeId,
    data: RwLock<ScopeMutData>,
    shutdown_handle: Option<ShutdownHandle>,
    abort_handle: Option<AbortHandle>,
    parent: Option<Scope>,
    children: RwLock<HashMap<ScopeId, Scope>>,
}

/// Scope data
#[derive(Debug, Default)]
pub struct ScopeMutData {
    created: HashMap<TypeId, Box<dyn CloneAny + Send + Sync>>,
    dependencies: HashMap<TypeId, Dependency>,
}

impl Scope {
    pub(crate) fn root(abort_handle: AbortHandle) -> Scope {
        Scope {
            inner: Arc::new(ScopeInner {
                id: ROOT_SCOPE,
                data: Default::default(),
                shutdown_handle: Default::default(),
                abort_handle: Some(abort_handle),
                parent: None,
                children: Default::default(),
            }),
            valid: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(crate) async fn child(
        &self,
        shutdown_handle: Option<ShutdownHandle>,
        abort_handle: Option<AbortHandle>,
    ) -> Self {
        log::trace!("Adding child to {:x}", self.id.as_fields().0);
        let id = Uuid::new_v4();
        let parent = self.clone();
        let child = Scope {
            inner: Arc::new(ScopeInner {
                id,
                data: RwLock::new(ScopeMutData {
                    created: Default::default(),
                    dependencies: Default::default(),
                }),
                shutdown_handle,
                abort_handle,
                parent: Some(parent),
                children: Default::default(),
            }),
            valid: Arc::new(AtomicBool::new(true)),
        };
        self.children.write().await.insert(id, child.clone());
        log::trace!("Added child to {:x}", self.id.as_fields().0);
        child
    }

    /// Find a scope by id
    pub(crate) fn find(&self, id: ScopeId) -> Option<&Scope> {
        if id == self.id {
            Some(self)
        } else {
            self.parent.as_ref().and_then(|p| p.find(id))
        }
    }

    pub(crate) fn parent(&self) -> Option<&Scope> {
        self.parent.as_ref()
    }

    pub(crate) async fn children(&self) -> Vec<Scope> {
        self.children.read().await.values().cloned().collect()
    }

    pub(crate) async fn drop(&self) {
        log::trace!("Dropping scope {:x}", self.id.as_fields().0);
        if let Some(parent) = self.parent.as_ref() {
            parent.children.write().await.remove(&self.id);
        }
        log::trace!("Dropped scope {:x}", self.id.as_fields().0);
    }

    pub(crate) async fn add_data(&self, data_type: TypeId, data: Box<dyn CloneAny + Send + Sync>) {
        if !self.valid.load(Ordering::Acquire) {
            log::warn!("Tried to add data to invalid scope {:x}", self.id.as_fields().0);
            return;
        }
        let mut scope_data = self.data.write().await;
        scope_data.created.insert(data_type, data);
        drop(scope_data);
        self.propagate_data(data_type, self).await;
    }

    pub(crate) async fn remove_data(&self, data_type: TypeId) -> Option<DepReady> {
        if !self.valid.load(Ordering::Acquire) {
            log::warn!("Tried to remove data from invalid scope {:x}", self.id.as_fields().0);
            return None;
        }
        let mut scope_data = self.data.write().await;
        if let Some(data) = scope_data.created.remove(&data_type) {
            if let Some(Dependency::Linked(_)) = scope_data.dependencies.remove(&data_type) {
                drop(scope_data);
                log::debug!(
                    "Aborting scope {:x} due to a removed critical dependency!",
                    self.id.as_fields().0,
                );
                self.abort().await;
            }
            Some(DepReady(data))
        } else {
            None
        }
    }

    #[async_recursion]
    async fn propagate_data(&self, data_type: TypeId, creator_scope: &Scope) {
        log::trace!("Propagating data to {:x}", self.id.as_fields().0);
        let mut scope_data = self.data.write().await;
        if let Some(dep) = scope_data.dependencies.remove(&data_type) {
            drop(scope_data);
            let data = creator_scope.data.read().await.created.get(&data_type).unwrap().clone();
            dep.get_signal().signal(data).await;

            if let Dependency::Linked(_) = dep {
                let mut scope_data = self.data.write().await;
                scope_data.dependencies.insert(data_type, dep);
                drop(scope_data);
            }
        } else {
            drop(scope_data);
        }
        let children = self.children().await;
        for child_scope in children {
            child_scope.propagate_data(data_type, creator_scope).await;
        }
        log::trace!("Propagated data to {:x}", self.id.as_fields().0);
    }

    /// Get some arbitrary data from the given scope
    pub(crate) async fn get_data(&self, data_type: TypeId) -> Option<DepReady> {
        if !self.valid.load(Ordering::Acquire) {
            log::warn!("Tried to get data from invalid scope {:x}", self.id.as_fields().0);
            return None;
        }
        let mut curr = Some(self);
        while let Some(scope) = curr {
            let scope_data = scope.data.read().await;
            let res = scope_data.created.get(&data_type).cloned();
            drop(scope_data);
            match res {
                Some(data) => return Some(DepReady(data)),
                None => curr = scope.parent.as_ref(),
            }
        }
        None
    }

    /// Get some arbitrary data from the given scope or a signal to await its creation
    pub(crate) async fn get_data_promise(&self, data_type: TypeId) -> Result<RawDepStatus, RuntimeError> {
        if !self.valid.load(Ordering::Acquire) {
            return Err(RuntimeError::InvalidScope);
        }
        let data = self.get_data(data_type).await;
        Ok(match data {
            Some(data) => RawDepStatus::Ready(data),
            None => {
                let mut scope_data = self.data.write().await;
                let flag = match scope_data.dependencies.entry(data_type) {
                    Entry::Occupied(o) => match o.get() {
                        Dependency::Once(f) | Dependency::Linked(f) => f.clone(),
                    },
                    Entry::Vacant(v) => {
                        let flag = DepSignal::default();
                        v.insert(Dependency::Once(flag.clone()));
                        flag
                    }
                };
                RawDepStatus::Waiting(flag)
            }
        })
    }

    pub(crate) async fn depend_on(&self, data_type: TypeId) -> Result<RawDepStatus, RuntimeError> {
        if !self.valid.load(Ordering::Acquire) {
            return Err(RuntimeError::InvalidScope);
        }
        let status = self.get_data(data_type).await;
        let mut scope_data = self.data.write().await;
        Ok(match scope_data.dependencies.entry(data_type) {
            Entry::Occupied(mut e) => {
                let val = e.get_mut();
                RawDepStatus::Waiting(val.upgrade().get_signal().clone())
            }
            Entry::Vacant(v) => {
                let flag = DepSignal::default();
                v.insert(Dependency::Linked(flag.clone()));
                drop(scope_data);
                if let Some(DepReady(t)) = status {
                    flag.signal(t).await;
                }
                RawDepStatus::Waiting(flag)
            }
        })
    }

    pub(crate) async fn shutdown(&self) {
        log::trace!("Shutting down scope {:x}", self.id.as_fields().0);
        self.valid.store(false, Ordering::Release);
        let data = self.data.write().await.dependencies.drain().collect::<Vec<_>>();
        for (_, dep) in data {
            dep.into_signal().cancel()
        }
        if let Some(handle) = self.shutdown_handle.as_ref() {
            handle.shutdown();
        } else if let Some(abort) = self.abort_handle.as_ref() {
            abort.abort();
        }
        log::trace!("Shut down scope {:x}", self.id.as_fields().0);
    }

    /// Abort the tasks in this scope.
    #[async_recursion]
    pub(crate) async fn abort(&self) {
        log::trace!("Aborting scope {:x}", self.id.as_fields().0);
        let data = self.data.write().await.dependencies.drain().collect::<Vec<_>>();
        for (_, dep) in data {
            dep.into_signal().cancel()
        }
        let children = self.children().await;
        for child_scope in children {
            child_scope.abort().await;
        }
        if let Some(handle) = self.shutdown_handle.as_ref() {
            handle.shutdown();
        }
        if let Some(abort) = self.abort_handle.as_ref() {
            abort.abort();
        }
        log::trace!("Aborted scope {:x}", self.id.as_fields().0);
    }
}

impl Deref for Scope {
    type Target = ScopeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone)]
enum Dependency {
    Once(DepSignal),
    Linked(DepSignal),
}

impl Dependency {
    pub fn upgrade(&mut self) -> &mut Self {
        match self {
            Dependency::Once(flag) => {
                *self = Dependency::Linked(std::mem::take(flag));
                self
            }
            Dependency::Linked(_) => self,
        }
    }

    pub fn get_signal(&self) -> &DepSignal {
        match self {
            Dependency::Once(flag) | Dependency::Linked(flag) => flag,
        }
    }

    pub fn into_signal(self) -> DepSignal {
        match self {
            Dependency::Once(flag) | Dependency::Linked(flag) => flag,
        }
    }
}

/// The status of a dependency
pub enum DepStatus<T> {
    /// The dependency is ready to be used
    Ready(T),
    /// The dependency is not ready, here is a flag to await
    Waiting(DepHandle<T>),
}

impl<T: 'static + Clone + Send + Sync> From<DepStatus<T>> for Option<T> {
    fn from(status: DepStatus<T>) -> Self {
        match status {
            DepStatus::Ready(t) => Some(t),
            DepStatus::Waiting(h) => {
                if h.flag.set.load(Ordering::Acquire) {
                    h.flag
                        .val
                        .try_read()
                        .ok()
                        .and_then(|lock| lock.clone().map(|d| *unsafe { d.clone().downcast_unchecked() }))
                } else {
                    None
                }
            }
        }
    }
}

impl<T: 'static + Clone + Send + Sync> DepStatus<T> {
    /// Wait for a dependency to become ready.
    /// Will return immediately if it is already ready.
    /// Will return an Err if the containing scope is dropped.
    pub async fn get(self) -> Result<T, RuntimeError> {
        match self {
            DepStatus::Ready(t) => Ok(t),
            DepStatus::Waiting(h) => h.await,
        }
    }

    /// Get the value of a dependency if it is ready, otherwise return None.
    pub fn get_opt(self) -> Option<T> {
        self.into()
    }
}

/// A dynamic dependency status
#[derive(Debug)]
pub(crate) enum RawDepStatus {
    /// The dependency is ready to be used
    Ready(DepReady),
    /// The dependency is not ready, here is a flag to await
    Waiting(DepSignal),
}

impl RawDepStatus {
    /// Convert this dynamic status to a typed one
    pub fn with_type<T: 'static + Clone + Send + Sync>(self) -> DepStatus<T> {
        match self {
            RawDepStatus::Ready(t) => DepStatus::Ready(t.with_type()),
            RawDepStatus::Waiting(s) => DepStatus::Waiting(s.handle()),
        }
    }
}

/// A ready dependency
#[derive(Debug)]
pub(crate) struct DepReady(Box<dyn CloneAny + Send + Sync>);

impl DepReady {
    /// Convert this ready dependency to a type
    pub fn with_type<T: 'static + Clone + Send + Sync>(self) -> T {
        *unsafe { self.0.downcast_unchecked() }
    }
}

#[derive(Default, Debug)]
pub(crate) struct DepFlag {
    waker: AtomicWaker,
    set: AtomicBool,
    val: RwLock<Option<Box<dyn CloneAny + Send + Sync>>>,
}

impl DepFlag {
    pub(crate) async fn signal(&self, val: Box<dyn CloneAny + Send + Sync>) {
        *self.val.write().await = Some(val);
        self.set.store(true, Ordering::Release);
        self.waker.wake();
    }

    pub(crate) fn cancel(&self) {
        self.set.store(true, Ordering::Release);
        self.waker.wake();
    }
}

/// A signal for a dependency
#[derive(Clone, Default, Debug)]
pub struct DepSignal {
    flag: Arc<DepFlag>,
}

impl DepSignal {
    pub(crate) async fn signal(&self, val: Box<dyn CloneAny + Send + Sync>) {
        self.flag.signal(val).await
    }

    pub(crate) fn cancel(self) {
        self.flag.cancel();
    }

    pub(crate) fn handle<T: 'static + Clone + Send + Sync>(self) -> DepHandle<T> {
        DepHandle {
            flag: self.flag,
            _type: PhantomData,
        }
    }
}

/// A handle to an awaitable dependency
#[derive(Clone, Default)]
pub struct DepHandle<T> {
    flag: Arc<DepFlag>,
    _type: PhantomData<fn(T) -> T>,
}

impl<T: 'static + Clone> Future for DepHandle<T> {
    type Output = Result<T, RuntimeError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // quick check to avoid registration if already done.
        if self.flag.set.load(Ordering::Acquire) {
            return match self.flag.val.try_read() {
                Ok(lock) => Poll::Ready(
                    lock.clone()
                        .ok_or(RuntimeError::CanceledDepNotification)
                        .map(|d| *unsafe { d.downcast_unchecked::<T>() }),
                ),
                Err(_) => Poll::Pending,
            };
        }

        self.flag.waker.register(cx.waker());

        // Need to check condition **after** `register` to avoid a race
        // condition that would result in lost notifications.
        if self.flag.set.load(Ordering::Acquire) {
            match self.flag.val.try_read() {
                Ok(lock) => Poll::Ready(
                    lock.clone()
                        .ok_or(RuntimeError::CanceledDepNotification)
                        .map(|d| *unsafe { d.downcast_unchecked::<T>() }),
                ),
                Err(_) => Poll::Pending,
            }
        } else {
            Poll::Pending
        }
    }
}
