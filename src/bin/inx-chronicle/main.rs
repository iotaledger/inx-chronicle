// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
mod error;
mod metrics;
mod process;
#[cfg(all(feature = "stardust", feature = "inx"))]
mod stardust_inx;

use bytesize::ByteSize;
use chronicle::db::MongoDb;
use clap::Parser;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

use crate::{cli::ClArgs, error::Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();
    set_up_logging();

    std::panic::set_hook(Box::new(|p| {
        error!("{}", p);
    }));

    let cl_args = ClArgs::parse();
    let config = cl_args.get_config()?;
    if cl_args.process_subcommands(&config)? {
        return Ok(());
    }

    info!("Connecting to database at bind address `{}`.", config.mongodb.conn_str);
    let db = MongoDb::connect(&config.mongodb).await?;
    debug!("Available databases: `{:?}`", db.get_databases().await?);
    info!(
        "Connected to database `{}` ({})",
        db.name(),
        ByteSize::b(db.size().await?)
    );

    #[cfg(feature = "stardust")]
    {
        use chronicle::db::collections;
        let start_indexes = db.get_index_names().await?;
        db.create_collection::<collections::OutputCollection>().await;
        db.create_collection::<collections::BlockCollection>().await;
        db.create_collection::<collections::LedgerUpdateCollection>().await;
        db.create_collection::<collections::MilestoneCollection>().await;
        db.collection::<collections::OutputCollection>()
            .create_indexes()
            .await?;
        db.collection::<collections::BlockCollection>().create_indexes().await?;
        db.collection::<collections::LedgerUpdateCollection>()
            .create_indexes()
            .await?;
        db.collection::<collections::MilestoneCollection>()
            .create_indexes()
            .await?;
        let end_indexes = db.get_index_names().await?;
        for (collection, indexes) in end_indexes {
            if let Some(old_indexes) = start_indexes.get(&collection) {
                let num_created = indexes.difference(old_indexes).count();
                if num_created > 0 {
                    info!("Created {} new indexes in {}", num_created, collection);
                    if tracing::enabled!(tracing::Level::DEBUG) {
                        for index in indexes.difference(old_indexes) {
                            debug!(" - {}", index);
                        }
                    }
                }
            } else {
                info!("Created {} new indexes in {}", indexes.len(), collection);
            }
        }
    }

    let mut tasks: JoinSet<Result<(), Error>> = JoinSet::new();

    let (shutdown_signal, _) = tokio::sync::broadcast::channel::<()>(1);

    #[cfg(all(feature = "inx", feature = "stardust"))]
    if config.inx.enabled {
        let mut worker = stardust_inx::InxWorker::new(&db, &config.inx);
        let mut handle = shutdown_signal.subscribe();
        tasks.spawn(async move {
            tokio::select! {
                _ = worker.run() => {},
                _ = handle.recv() => {},
            }
            Ok(())
        });
    }

    #[cfg(feature = "api")]
    if config.api.enabled {
        use futures::FutureExt;
        let mut handle = shutdown_signal.subscribe();
        tasks.spawn(async move {
            let worker = api::ApiWorker::new(&db, &config.api).map_err(config::ConfigError::Api)?;
            worker.run(handle.recv().then(|_| async {})).await?;
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
        res = tasks.join_next() => {
            if let Some(Ok(Err(err))) = res {
                tracing::error!("A worker failed with error: {err}");
            }
        },
    }

    shutdown_signal.send(())?;

    // Allow the user to abort if the tasks aren't shutting down quickly.
    tokio::select! {
        _ = process::interupt_or_terminate() => {
            tracing::info!("received second ctrl-c or terminate - aborting");
            tasks.shutdown().await;
            tracing::info!("Abort successful");
        },
        _ = async { while tasks.join_next().await.is_some() {} } => {
            tracing::info!("Shutdown successful");
        },
    }

    Ok(())
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
