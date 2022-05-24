// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use async_trait::async_trait;

#[async_trait]
pub trait Task: Send + Sync + Sized {
    type Error: Error + Send + Sync;

    async fn run(self) -> Result<(), Self::Error>;
}
