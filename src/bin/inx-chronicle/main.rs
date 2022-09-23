// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
mod launcher;
mod metrics;
#[cfg(all(feature = "stardust", feature = "inx"))]
mod stardust_inx;

use std::error::Error;

use chronicle::runtime::{Runtime, RuntimeScope};
use launcher::Launcher;
use tracing::error;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    set_up_logging();

    std::panic::set_hook(Box::new(|p| {
        error!("{}", p);
    }));

    if let Err(e) = Runtime::launch(startup).await {
        error!("{}", e);
    }
}

fn set_up_logging() {
    #[cfg(feature = "opentelemetry")]
    {
        use tracing_subscriber::prelude::*;

        let tracer = opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name("Chronicle")
            .install_batch(opentelemetry::runtime::Tokio)
            .unwrap();

        let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
        .with(opentelemetry)
        // This filter should not exist, but if I remove it,
        // it causes the buffer to overflow
        .with(EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                // The filter should only be on the console logs
                //.with_filter(EnvFilter::from_default_env()),
        )
        .init();
    }
    #[cfg(not(feature = "opentelemetry"))]
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

async fn startup(scope: &mut RuntimeScope) -> Result<(), Box<dyn Error + Send + Sync>> {
    let launcher_addr = scope.spawn_actor_unsupervised(Launcher).await;

    tokio::spawn(async move {
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
