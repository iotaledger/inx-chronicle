// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub(super) mod error;
pub(super) mod report;

use std::{borrow::Cow, error::Error};

use async_trait::async_trait;

#[async_trait]
pub trait Task: Send + Sync + Sized {
    type Error: Error + Send + Sync;

    /// Set this task's name, primarily for debugging purposes.
    fn name(&self) -> Cow<'static, str> {
        std::any::type_name::<Self>().into()
    }

    async fn run(&mut self) -> Result<(), Self::Error>;
}
