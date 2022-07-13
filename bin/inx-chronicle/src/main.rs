// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

// TODO: This is currently broken, but may be fixed, so we should check it every once in a while
#![allow(clippy::needless_borrow)]

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

    spawn_task("shutdown listener", async move {
        shutdown_signal_listener().await;
        launcher_addr.abort().await;
    });

    Ok(())
}

async fn shutdown_signal_listener() {
    #[cfg(unix)]
    {
        use futures::future;
        use tokio::signal::unix::{signal, Signal, SignalKind};

        // Panic: none of the possible error conditions should happen.
        let mut signals = vec![SignalKind::interrupt(), SignalKind::terminate()]
            .iter()
            .map(|kind| signal(*kind).unwrap())
            .collect::<Vec<Signal>>();
        let signal_futs = signals.iter_mut().map(|signal| Box::pin(signal.recv()));
        let (signal_event, _, _) = future::select_all(signal_futs).await;

        if signal_event.is_none() {
            panic!("Shutdown signal stream failed, channel may have closed.");
        }
    }
    #[cfg(not(unix))]
    {
        if let Err(e) = tokio::signal::ctrl_c().await {
            panic!("Failed to intercept CTRL-C: {:?}.", e);
        }
    }
}
