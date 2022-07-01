// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod check_health;
mod cli;
mod config;
mod launcher;
#[cfg(feature = "metrics")]
mod metrics;
#[cfg(all(feature = "stardust", feature = "inx"))]
mod stardust_inx;

use std::error::Error;

use chronicle::runtime::{spawn_task, Runtime, RuntimeScope};
use launcher::Launcher;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    #[cfg(all(tokio_unstable, feature = "console"))]
    console_subscriber::init();

    std::panic::set_hook(Box::new(|p| {
        log::error!("{}", p);
    }));

    if let Err(e) = Runtime::launch(startup).await {
        log::error!("{}", e);
    }
}

async fn startup(scope: &mut RuntimeScope) -> Result<(), Box<dyn Error + Send + Sync>> {
    let launcher_addr = scope.spawn_actor_unsupervised(Launcher).await;

    spawn_task("ctrl-c listener", async move {
        tokio::signal::ctrl_c().await.ok();
        launcher_addr.abort().await;
    });

    Ok(())
}
