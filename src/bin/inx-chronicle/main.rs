// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that holds the entry point of the Chronicle application.

/// Module containing the API.
#[cfg(feature = "api")]
mod api;
// mod cli;
// mod config;
#[cfg(feature = "inx")]
mod inx;
// mod migrations;
mod process;

// use bytesize::ByteSize;
// use chronicle::db::MongoDb;
// use clap::Parser;
// use tokio::task::JoinSet;
// use tracing::{debug, error, info};
// use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// use self::{
//     cli::{ClArgs, PostCommand},
//     migrations::check_migration_version,
// };

// #[tokio::main]
// async fn main() -> eyre::Result<()> {
//     dotenvy::dotenv().ok();

//     let cl_args = ClArgs::parse();
//     let config = cl_args.get_config();

//     set_up_logging()?;

//     if cl_args.process_subcommands(&config).await? == PostCommand::Exit {
//         return Ok(());
//     }

//     info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
//     let db = MongoDb::connect(&config.mongodb).await?;
//     debug!("Available databases: `{:?}`", db.get_databases().await?);
//     info!(
//         "Connected to database `{}` ({})",
//         db.name(),
//         ByteSize::b(db.size().await?)
//     );

//     // check_migration_version(&db).await?;

//     #[cfg(feature = "inx")]
//     build_indexes(&db).await?;

//     let mut tasks: JoinSet<eyre::Result<()>> = JoinSet::new();

//     let (shutdown_signal, _) = tokio::sync::broadcast::channel::<()>(1);

//     #[cfg(feature = "inx")]
//     if config.inx.enabled {
//         #[cfg(feature = "influx")]
//         #[allow(unused_mut)]
//         let mut influx_required = false;
//         #[cfg(feature = "analytics")]
//         {
//             influx_required |= config.influxdb.analytics_enabled;
//         }
//         #[cfg(feature = "metrics")]
//         {
//             influx_required |= config.influxdb.metrics_enabled;
//         }

//         #[cfg(feature = "influx")]
//         let influx_db = if influx_required {
//             info!("Connecting to influx at `{}`", config.influxdb.url);
//             let influx_db = chronicle::db::influxdb::InfluxDb::connect(&config.influxdb).await?;
//             #[cfg(feature = "analytics")]
//             info!(
//                 "Connected to influx database `{}`",
//                 influx_db.analytics().database_name()
//             );
//             #[cfg(feature = "metrics")]
//             info!("Connected to influx database `{}`", influx_db.metrics().database_name());
//             Some(influx_db)
//         } else {
//             None
//         };

//         let mut worker = inx::InxWorker::new(db.clone(), config.inx.clone());
//         #[cfg(feature = "influx")]
//         if let Some(influx_db) = &influx_db {
//             worker.set_influx_db(influx_db);
//         }

//         let mut handle = shutdown_signal.subscribe();
//         tasks.spawn(async move {
//             tokio::select! {
//                 res = worker.run() => res?,
//                 _ = handle.recv() => {},
//             }
//             Ok(())
//         });
//     }

//     #[cfg(feature = "api")]
//     if config.api.enabled {
//         use futures::FutureExt;
//         let worker = api::ApiWorker::new(db.clone(), config.api.clone())?;
//         let mut handle = shutdown_signal.subscribe();
//         tasks.spawn(async move {
//             worker.run(handle.recv().then(|_| async {})).await?;
//             Ok(())
//         });
//     }

//     let mut exit_code = Ok(());

//     // We wait for either the interrupt signal to arrive or for a component of our system to signal a shutdown.
//     tokio::select! {
//         res = process::interrupt_or_terminate() => {
//             if let Err(err) = res {
//                 tracing::error!("subscribing to OS interrupt signals failed with error: {err}; shutting down");
//                 exit_code = Err(err);
//             } else {
//                 tracing::info!("received ctrl-c or terminate; shutting down");
//             }
//         },
//         res = tasks.join_next() => {
//             if let Some(Ok(Err(err))) = res {
//                 tracing::error!("a worker failed with error: {err}");
//                 exit_code = Err(err);
//             }
//         },
//     }

//     shutdown_signal.send(())?;

//     // Allow the user to abort if the tasks aren't shutting down quickly.
//     tokio::select! {
//         res = process::interrupt_or_terminate() => {
//             if let Err(err) = res {
//                 tracing::error!("subscribing to OS interrupt signals failed with error: {err}; aborting");
//                 exit_code = Err(err);
//             } else {
//                 tracing::info!("received second ctrl-c or terminate; aborting");
//             }
//             tasks.shutdown().await;
//             tracing::info!("runtime aborted");
//         },
//         _ = async { while tasks.join_next().await.is_some() {} } => {
//             tracing::info!("runtime stopped");
//         },
//     }

//     exit_code
// }

// fn set_up_logging() -> eyre::Result<()> {
//     std::panic::set_hook(Box::new(|p| {
//         error!("{}", p);
//     }));

//     let registry = tracing_subscriber::registry();

//     let registry = {
//         registry
//             .with(EnvFilter::from_default_env())
//             .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE))
//     };

//     registry.init();
//     Ok(())
// }

// async fn build_indexes(db: &MongoDb) -> eyre::Result<()> {
//     use chronicle::db::mongodb::collections;
//     let start_indexes = db.get_index_names().await?;
//     db.create_indexes::<collections::OutputCollection>().await?;
//     db.create_indexes::<collections::BlockCollection>().await?;
//     db.create_indexes::<collections::LedgerUpdateCollection>().await?;
//     db.create_indexes::<collections::MilestoneCollection>().await?;
//     let end_indexes = db.get_index_names().await?;
//     for (collection, indexes) in end_indexes {
//         if let Some(old_indexes) = start_indexes.get(&collection) {
//             let num_created = indexes.difference(old_indexes).count();
//             if num_created > 0 {
//                 info!("Created {} new indexes in {}", num_created, collection);
//                 if tracing::enabled!(tracing::Level::DEBUG) {
//                     for index in indexes.difference(old_indexes) {
//                         debug!(" - {}", index);
//                     }
//                 }
//             }
//         } else {
//             info!("Created {} new indexes in {}", indexes.len(), collection);
//         }
//     }
//     Ok(())
// }

fn main() {}
