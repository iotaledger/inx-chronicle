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
#[cfg(all(feature = "stardust", feature = "inx"))]
mod stardust_inx;

use std::error::Error;

use chronicle::runtime::{spawn_task, Runtime, RuntimeScope};
use launcher::Launcher;
use tracing::error;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    set_up_logging();
    #[cfg(all(tokio_unstable, feature = "console"))]
    console_subscriber::init();

    std::panic::set_hook(Box::new(|p| {
        error!("{}", p);
    }));

    if let Err(e) = Runtime::launch(startup).await {
        error!("{}", e);
    }
}

fn set_up_logging() {
    use tracing_subscriber::prelude::*;

    #[cfg(feature = "metrics")]
    {
        // use opentelemetry::sdk::trace as sdktrace;
        // let tracer = opentelemetry_otlp::new_pipeline()
        //    .tracing()
        //    .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        //    .with_trace_config(sdktrace::config().with_sampler(sdktrace::Sampler::AlwaysOn))
        //    .install_simple()
        //    .expect("Unable to initialize OtlpPipeline");

        let tracer = opentelemetry::sdk::export::trace::stdout::new_pipeline().install_simple();

        let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(opentelemetry)
            .init();
    }
    #[cfg(not(feature = "metrics"))]
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
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
