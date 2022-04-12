// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use async_recursion::async_recursion;
use futures::future::AbortHandle;
use tokio::sync::RwLock;
pub use uuid::Uuid;

use super::shutdown::ShutdownHandle;

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
    shutdown_handle: Option<ShutdownHandle>,
    abort_handle: Option<AbortHandle>,
    parent: Option<Scope>,
    children: RwLock<HashMap<ScopeId, Scope>>,
}

impl Scope {
    pub(crate) fn root(abort_handle: AbortHandle) -> Scope {
        Scope {
            inner: Arc::new(ScopeInner {
                id: ROOT_SCOPE,
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

    pub(crate) async fn shutdown(&self) {
        log::trace!("Shutting down scope {:x}", self.id.as_fields().0);
        self.valid.store(false, Ordering::Release);
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
