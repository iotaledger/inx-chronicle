// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The entry point to Chronicle.

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
mod error;
mod launcher;
mod metrics;
mod process;
#[cfg(all(feature = "stardust", feature = "inx"))]
mod stardust_inx;
mod shutdown;

use std::error::Error;

use bytesize::ByteSize;
use chronicle::{runtime::{Runtime, RuntimeScope}, db::MongoDb};
use launcher::{Launcher, LauncherError};
use clap::Parser;
use tokio::task::JoinSet;
use tracing::{info, error, debug, log::warn};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

use crate::{cli::ClArgs, config::{ChronicleConfig, ConfigError}};

#[tokio::main]
async fn main() -> Result<(), LauncherError> {
    dotenv::dotenv().ok();
    set_up_logging();

    std::panic::set_hook(Box::new(|p| {
        error!("{}", p);
    }));

    let (mut shutdown_from_app, shutdown_notifier, shutdown_signal) = shutdown::shutdown_handles();
/////////

let cl_args = ClArgs::parse();

        let mut config = cl_args.config.as_ref().map(ChronicleConfig::from_file).transpose()?.unwrap_or_default();
        config.apply_cl_args(&cl_args);

        info!(
            "Connecting to database at bind address `{}`.",
            config.mongodb.connect_url
        );
        let db = MongoDb::connect(&config.mongodb).await?;
        debug!("Available databases: `{:?}`", db.get_databases().await?);
        info!(
            "Connected to database `{}` ({})",
            db.name(),
            ByteSize::b(db.size().await?)
        );

        #[cfg(feature = "stardust")]
        {
            db.collection::<chronicle::db::collections::OutputCollection>()
                .create_indexes()
                .await?;
            db.collection::<chronicle::db::collections::BlockCollection>()
                .create_indexes()
                .await?;
            db.collection::<chronicle::db::collections::LedgerUpdateCollection>()
                .create_indexes()
                .await?;
            db.collection::<chronicle::db::collections::MilestoneCollection>()
                .create_indexes()
                .await?;
        }

        let mut tasks: JoinSet<Result<(), error::Error>> = JoinSet::new();

        #[cfg(all(feature = "inx", feature = "stardust"))]
        if config.inx.enabled {
            let shutdown_signal = shutdown_signal.clone();
            let worker = stardust_inx::InxWorker::new(&db, &config.inx);
            tasks.spawn(async move {
                worker.start(shutdown_signal).await?;
                Ok(())
            });
        }

        #[cfg(feature = "api")]
        if config.api.enabled {
            tasks.spawn(async move {
                let worker = api::ApiWorker::new(&db, &config.api).map_err(ConfigError::Api)?;
                worker.start(shutdown_signal).await?;
                Ok(())
            });
        }

        if config.metrics.enabled {
            if let Err(err) = crate::metrics::setup(&config.metrics) {
                warn!("Failed to build Prometheus exporter: {err}");
            } else {
                info!(
                    "Exporting to Prometheus at bind address: {}:{}",
                    config.metrics.address, config.metrics.port
                );
            };
        }


    /////////

    if let Err(e) = Runtime::launch(startup).await {
        error!("{}", e);
    }

    // We wait for either the interrupt signal to arrive or for a component of our system to signal a shutdown.
    tokio::select! {
        _ = process::interupt_or_terminate() => {
            tracing::info!("received ctrl-c or terminate");
        },
        _ = shutdown_from_app.recv() => {
            tracing::info!("received shutdown signal from component");
        },
        // TODO: better error handling/reporting
        _ = tasks.join_next() => (),
        // res = inx_handle => {
        //     match res.expect("joining the handle should not fail.") {
        //         Ok(_) => (),
        //         Err(err) => tracing::error!("INX task failed with error: {err}"),
        //     }
        // },
    }

    // We send the shutdown signal to all tasks that have an instance of the `shutdown_signal`.
    shutdown_notifier.emit().await;

    tracing::info!("shutdown complete");

    Ok(())
}

fn set_up_logging() {
    #[cfg(feature = "opentelemetry")]
    {
        use tracing_subscriber::prelude::*;

        let tracer = opentelemetry_jaeger::new_pipeline()
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

#[deprecated]
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
