// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that holds the entry point of the Chronicle application.

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
mod cli;
mod config;
mod migrations;
mod process;
#[cfg(feature = "inx")]
mod stardust_inx;

use bytesize::ByteSize;
use chronicle::db::{collections::ApplicationStateCollection, MongoDb};
use clap::Parser;
use tokio::task::JoinSet;
use tracing::{debug, error, info};
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use self::cli::{ClArgs, PostCommand};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let cl_args = ClArgs::parse();
    let config = cl_args.get_config();

    set_up_logging()?;

    if cl_args.process_subcommands(&config).await? == PostCommand::Exit {
        return Ok(());
    }

    info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
    let db = MongoDb::connect(&config.mongodb).await?;
    debug!("Available databases: `{:?}`", db.get_databases().await?);
    info!(
        "Connected to database `{}` ({})",
        db.name(),
        ByteSize::b(db.size().await?)
    );

    match db
        .collection::<ApplicationStateCollection>()
        .get_last_migration()
        .await?
        .as_deref()
    {
        None => {
            info!("Setting migration version to {}", migrations::LATEST_VERSION);
            db.collection::<ApplicationStateCollection>()
                .set_last_migration(migrations::LATEST_VERSION)
                .await?;
        }
        Some(migrations::LATEST_VERSION) => (),
        Some(v) => {
            eyre::bail!(
                "Invalid database migration version {}, please run the `migrate` command.",
                v
            );
        }
    }

    build_indexes(&db).await?;

    let mut tasks: JoinSet<eyre::Result<()>> = JoinSet::new();

    let (shutdown_signal, _) = tokio::sync::broadcast::channel::<()>(1);

    #[cfg(feature = "inx")]
    if config.inx.enabled {
        #[cfg(any(feature = "analytics", feature = "metrics"))]
        #[allow(unused_mut)]
        let mut influx_required = false;
        #[cfg(feature = "analytics")]
        {
            influx_required |= config.influxdb.analytics_enabled;
        }
        #[cfg(feature = "metrics")]
        {
            influx_required |= config.influxdb.metrics_enabled;
        }

        #[cfg(any(feature = "analytics", feature = "metrics"))]
        let influx_db = if influx_required {
            info!("Connecting to influx at `{}`", config.influxdb.url);
            let influx_db = chronicle::db::influxdb::InfluxDb::connect(&config.influxdb).await?;
            #[cfg(feature = "analytics")]
            info!(
                "Connected to influx database `{}`",
                influx_db.analytics().database_name()
            );
            #[cfg(feature = "metrics")]
            info!("Connected to influx database `{}`", influx_db.metrics().database_name());
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

fn set_up_logging() -> eyre::Result<()> {
    std::panic::set_hook(Box::new(|p| {
        error!("{}", p);
    }));

    let registry = tracing_subscriber::registry();

    let registry = {
        registry
            .with(EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE))
    };

    registry.init();
    Ok(())
}

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
