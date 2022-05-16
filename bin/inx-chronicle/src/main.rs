// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
#[cfg(feature = "inx")]
mod inx;
mod launcher;

use std::error::Error;

use chronicle::runtime::{scope::RuntimeScope, Runtime};
use launcher::Launcher;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    std::panic::set_hook(Box::new(|p| {
        log::error!("{}", p);
    }));

    if let Err(e) = Runtime::launch(startup).await {
        log::error!("{}", e);
    }
}

async fn startup(scope: &mut RuntimeScope) -> Result<(), Box<dyn Error + Send + Sync>> {
    let launcher_addr = scope.spawn_actor_unsupervised(Launcher::default()).await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        launcher_addr.shutdown();
    });

    Ok(())
}
