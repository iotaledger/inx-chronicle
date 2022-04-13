// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use futures::{
    future::{AbortHandle, Abortable},
    Future,
};

use self::{error::RuntimeError, scope::RuntimeScope};

/// Module containing the Actor trait and associated types.
pub mod actor;
/// Module containing runtime spawn configurations.
pub mod config;
/// Module containing runtime errors.
pub mod error;
mod registry;
/// Module containing runtime scope types.
pub mod scope;
mod shutdown;

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
        log::debug!("Spawning runtime");
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
