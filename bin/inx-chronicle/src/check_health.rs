// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::runtime::{Actor, HandleEvent, RuntimeError, ScopeView, Sender};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum CheckHealthError {
    #[error(transparent)]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

pub(crate) struct IsHealthy(pub tokio::sync::oneshot::Sender<bool>);

#[async_trait]
pub(crate) trait CheckHealth {
    async fn is_healthy<A: Actor>(&self) -> Result<bool, CheckHealthError>
    where
        A: 'static + HandleEvent<IsHealthy>;
}

#[async_trait]
impl CheckHealth for ScopeView {
    async fn is_healthy<A: Actor>(&self) -> Result<bool, CheckHealthError>
    where
        A: 'static + HandleEvent<IsHealthy>,
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.addr::<A>().await.send(IsHealthy(sender))?;
        Ok(receiver.await?)
    }
}
