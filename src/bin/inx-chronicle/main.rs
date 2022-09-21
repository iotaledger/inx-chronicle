// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The entry point to Chronicle.

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
mod error;
mod metrics;
mod process;
mod shutdown;
#[cfg(all(feature = "stardust", feature = "inx"))]
mod stardust_inx;

use bytesize::ByteSize;
use chronicle::db::MongoDb;
use clap::Parser;
use tokio::task::JoinSet;
use tracing::{debug, error, info, log::warn};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

use crate::{
    cli::ClArgs,
    config::{ChronicleConfig},
    error::Error,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    set_up_logging();

    std::panic::set_hook(Box::new(|p| {
        error!("{}", p);
    }));

    let (mut shutdown_from_app, shutdown_notifier, shutdown_signal) = shutdown::shutdown_handles();
    /////////

    let cl_args = ClArgs::parse();

    let mut config = cl_args
        .config
        .as_ref()
        .map(ChronicleConfig::from_file)
        .transpose()?
        .unwrap_or_default();
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
        let mut worker = stardust_inx::InxWorker::new(&db, &config.inx);
        tasks.spawn(async move {
            worker.run(shutdown_signal).await?;
            Ok(())
        });
    }

    #[cfg(feature = "api")]
    if config.api.enabled {
        tasks.spawn(async move {
            let worker = api::ApiWorker::new(&db, &config.api).map_err(config::ConfigError::Api)?;
            worker.run(shutdown_signal).await?;
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

    // We wait for either the interrupt signal to arrive or for a component of our system to signal a shutdown.
    tokio::select! {
        _ = process::interupt_or_terminate() => {
            tracing::info!("received ctrl-c or terminate");
        },
        _ = shutdown_from_app.recv() => {
            tracing::info!("received shutdown signal from component");
        },
        res = tasks.join_next() => {
            match res {
                Some(Ok(Err(err))) => tracing::error!("A worker failed with error: {err}"),
                _ => {},
            }
        },
    }

    // We send the shutdown signal to all tasks that have an instance of the `shutdown_signal`.
    shutdown_notifier.emit().await;

    tracing::info!("Shutdown successful.");

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
