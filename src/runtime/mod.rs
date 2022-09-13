// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing the Actor trait and associated types.
mod actor;
/// Module containing runtime spawn configurations.
mod config;
/// Module containing runtime errors.
mod error;
/// Module containing stream merge functionality.
mod merge;
/// Module containing the actor registry.
mod registry;
/// Module containing runtime scope types.
mod scope;
/// Module containing shutdown functionality.
mod shutdown;
mod task;

use std::error::Error;

use futures::{
    future::{AbortHandle, Abortable},
    Future,
};
use tracing::debug;

pub use self::{
    actor::{
        addr::Addr,
        context::ActorContext,
        error::ActorError,
        event::HandleEvent,
        report::Report,
        sender::{IsClosed, Sender},
        util::SpawnActor,
        Actor,
    },
    config::{ConfigureActor, SpawnConfig},
    error::{ErrorLevel, RuntimeError},
    scope::{RuntimeScope, ScopeView},
    task::{error::TaskError, report::TaskReport, Task},
};

#[allow(missing_docs)]
pub trait AsyncFn<'a, O> {
    type Output: 'a + Future<Output = O> + Send;
    fn call(self, cx: &'a mut RuntimeScope) -> Self::Output;
}
impl<'a, Fn, Fut, O> AsyncFn<'a, O> for Fn
where
    Fn: FnOnce(&'a mut RuntimeScope) -> Fut,
    Fut: 'a + Future<Output = O> + Send,
{
    type Output = Fut;
    fn call(self, cx: &'a mut RuntimeScope) -> Self::Output {
        (self)(cx)
    }
}

/// Starting point for the runtime.
pub struct Runtime;

impl Runtime {
    /// Launches a new root runtime scope.
    pub async fn launch<F>(f: F) -> Result<(), RuntimeError>
    where
        for<'a> F: AsyncFn<'a, Result<(), Box<dyn Error + Send + Sync>>>,
    {
        debug!("Spawning runtime");
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let mut scope = RuntimeScope::root(abort_handle);
        let res = Abortable::new(f.call(&mut scope), abort_registration).await;
        if let Ok(Err(_)) = res {
            scope.abort().await;
        }
        scope.join().await;
        match res {
            Ok(res) => res.map_err(|e| RuntimeError::ScopeLaunchError(e)),
            Err(_) => Err(RuntimeError::AbortedScope(scope.id())),
        }
    }
}
