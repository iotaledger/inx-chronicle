// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that holds the entry point of the Chronicle application.

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
mod process;
#[cfg(all(feature = "stardust", feature = "inx"))]
mod stardust_inx;

use bytesize::ByteSize;
use chronicle::db::MongoDb;
use clap::Parser;
use config::ChronicleConfig;
use tokio::task::JoinSet;
use tracing::{debug, error, info};
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use self::cli::{ClArgs, PostCommand};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    std::panic::set_hook(Box::new(|p| {
        error!("{}", p);
    }));

    let cl_args = ClArgs::parse();
    let config = cl_args.get_config()?;
    if cl_args.process_subcommands(&config).await? == PostCommand::Exit {
        return Ok(());
    }

    set_up_logging(&config)?;

    info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
    let db = MongoDb::connect(&config.mongodb).await?;
    debug!("Available databases: `{:?}`", db.get_databases().await?);
    info!(
        "Connected to database `{}` ({})",
        db.name(),
        ByteSize::b(db.size().await?)
    );

    #[cfg(feature = "stardust")]
    build_indexes(&db).await?;

    let mut tasks: JoinSet<eyre::Result<()>> = JoinSet::new();

    let (shutdown_signal, _) = tokio::sync::broadcast::channel::<()>(1);

    #[cfg(all(feature = "inx", feature = "stardust"))]
    if config.inx.enabled {
        #[cfg(any(feature = "analytics", feature = "metrics"))]
        let influx_db = if config.influxdb.analytics_enabled || config.influxdb.metrics_enabled {
            info!("Connecting to influx database at address `{}`", config.influxdb.url);
            let influx_db = chronicle::db::influxdb::InfluxDb::connect(&config.influxdb).await?;
            info!("Connected to influx database `{}`", influx_db.database_name());
            Some(influx_db)
        } else {
            None
        };

        let mut worker = stardust_inx::InxWorker::new(
            &db,
            #[cfg(any(feature = "analytics", feature = "metrics"))]
            influx_db.as_ref(),
            &config.inx,
        );
        let mut handle = shutdown_signal.subscribe();
        tasks.spawn(async move {
            tokio::select! {
                res = worker.run() => {
                    res?;
                },
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
            let worker = api::ApiWorker::new(&db, &config.api)?;
            worker.run(handle.recv().then(|_| async {})).await?;
            Ok(())
        });
    }

    // We wait for either the interrupt signal to arrive or for a component of our system to signal a shutdown.
    tokio::select! {
        _ = process::interrupt_or_terminate() => {
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
        _ = process::interrupt_or_terminate() => {
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

fn set_up_logging(#[allow(unused)] config: &ChronicleConfig) -> eyre::Result<()> {
    let registry = tracing_subscriber::registry();

    #[cfg(feature = "opentelemetry")]
    let registry = {
        let tracer = opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name("Chronicle")
            .install_batch(opentelemetry::runtime::Tokio)
            .unwrap();

        registry
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            // This filter should not exist, but if I remove it,
            // it causes the buffer to overflow
            .with(EnvFilter::from_default_env())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_span_events(FmtSpan::CLOSE)
                    // The filter should only be on the console logs
                    //.with_filter(EnvFilter::from_default_env()),
            )
    };
    #[cfg(not(feature = "opentelemetry"))]
    let registry = {
        registry
            .with(EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE))
    };
    #[cfg(feature = "loki")]
    let registry = {
        let (layer, task) = tracing_loki::layer(config.loki.connect_url.parse()?, [].into(), [].into())?;
        tokio::spawn(task);
        registry.with(layer)
    };

    registry.init();
    Ok(())
}

#[cfg(feature = "stardust")]
async fn build_indexes(db: &MongoDb) -> eyre::Result<()> {
    use chronicle::db::collections;
    let start_indexes = db.get_index_names().await?;
    db.create_indexes::<collections::OutputCollection>().await?;
    db.create_indexes::<collections::BlockCollection>().await?;
    db.create_indexes::<collections::LedgerUpdateCollection>().await?;
    db.create_indexes::<collections::MilestoneCollection>().await?;
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
    Ok(())
}
