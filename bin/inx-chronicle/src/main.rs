// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
mod launcher;
#[cfg(feature = "metrics")]
mod metrics;
mod shutdown;
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

    #[cfg(unix)]
    let shutdown_rx = shutdown::shutdown_listener(vec![
        tokio::signal::unix::SignalKind::interrupt(),
        tokio::signal::unix::SignalKind::terminate(),
    ]);

    #[cfg(not(unix))]
    let shutdown_rx = shutdown::shutdown_listener();

    spawn_task("shutdown listener", async move {
        if let Err(e) = shutdown_rx.await {
            log::warn!("awaiting shutdown failed: {:?}", e);
        }
        launcher_addr.abort().await;
    });

    Ok(())
}
