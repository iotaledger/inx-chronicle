// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub(super) mod error;
pub(super) mod report;

use std::borrow::Cow;

use async_trait::async_trait;

use super::error::ErrorLevel;

/// The task trait, which defines a task that is managed by the runtime and requires little to no
/// external comunication.
#[async_trait]
pub trait Task: Send + Sync + Sized {
    /// Custom error type that is returned by all task methods.
    type Error: ErrorLevel + Send;

    /// Set this task's name, primarily for debugging purposes.
    fn name(&self) -> Cow<'static, str> {
        std::any::type_name::<Self>().into()
    }

    /// Run the task.
    async fn run(&mut self) -> Result<(), Self::Error>;
}
