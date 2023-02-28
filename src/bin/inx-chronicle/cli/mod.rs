// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::mongodb::config as mongodb;
use clap::{Args, Parser, Subcommand};

use crate::config::ChronicleConfig;

#[cfg(feature = "analytics")]
pub mod analytics;
#[cfg(feature = "api")]
mod api;
#[cfg(feature = "influx")]
mod influx;
#[cfg(feature = "inx")]
mod inx;

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
// #[command(author, version, about, next_display_order = None)]
#[command(author, version, about)]
pub struct ClArgs {
    /// MongoDb arguments.
    #[command(flatten, next_help_heading = "MongoDb")]
    pub mongodb: MongoDbArgs,
    /// InfluxDb arguments.
    #[cfg(feature = "influx")]
    #[command(flatten, next_help_heading = "InfluxDb")]
    pub influxdb: influx::InfluxDbArgs,
    /// INX arguments.
    #[cfg(feature = "inx")]
    #[command(flatten, next_help_heading = "INX")]
    pub inx: inx::InxArgs,
    /// Rest API arguments.
    #[cfg(feature = "api")]
    #[command(flatten, next_help_heading = "API")]
    pub api: api::ApiArgs,
    /// Subcommands.
    #[command(subcommand)]
    pub subcommand: Option<Subcommands>,
}

#[derive(Args, Debug)]
pub struct MongoDbArgs {
    /// The MongoDb connection string.
    #[arg(
        long,
        value_name = "CONN_STR",
        env = "MONGODB_CONN_STR",
        default_value = mongodb::DEFAULT_CONN_STR,
    )]
    pub mongodb_conn_str: String,
    /// The MongoDb database name.
    #[arg(long, value_name = "NAME", env = "MONGODB_DB_NAME", default_value = mongodb::DEFAULT_DATABASE_NAME)]
    pub mongodb_database_name: String,
}

impl From<&MongoDbArgs> for chronicle::db::MongoDbConfig {
    fn from(value: &MongoDbArgs) -> Self {
        Self {
            conn_str: value.mongodb_conn_str.clone(),
            database_name: value.mongodb_database_name.clone(),
        }
    }
}

impl ClArgs {
    /// Creates a [`ChronicleConfig`] from the given command-line arguments, environment variables, and defaults.
    pub fn get_config(&self) -> ChronicleConfig {
        ChronicleConfig {
            mongodb: (&self.mongodb).into(),
            #[cfg(feature = "influx")]
            influxdb: (&self.influxdb).into(),
            #[cfg(feature = "inx")]
            inx: (&self.inx).into(),
            #[cfg(feature = "api")]
            api: (&self.api).into(),
        }
    }

    /// Process subcommands and return whether the app should early exit.
    #[allow(unreachable_patterns)]
    #[allow(clippy::collapsible_match)]
    pub async fn process_subcommands(&self, config: &ChronicleConfig) -> eyre::Result<PostCommand> {
        if let Some(subcommand) = &self.subcommand {
            match subcommand {
                #[cfg(feature = "api")]
                Subcommands::GenerateJWT(cmd) => {
                    cmd.handle(&config.api)?;
                }
                #[cfg(feature = "analytics")]
                Subcommands::FillAnalytics(cmd) => {
                    cmd.handle(config).await?;
                }
                #[cfg(feature = "analytics")]
                Subcommands::FillIntervalAnalytics(cmd) => {
                    cmd.handle(config).await?;
                }
                #[cfg(debug_assertions)]
                Subcommands::ClearDatabase { run } => {
                    tracing::info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
                    let db = chronicle::db::MongoDb::connect(&config.mongodb).await?;
                    db.clear().await?;
                    tracing::info!("Database cleared successfully.");
                    if *run {
                        return Ok(PostCommand::Start);
                    }
                }
                Subcommands::BuildIndexes => {
                    tracing::info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
                    let db = chronicle::db::MongoDb::connect(&config.mongodb).await?;
                    super::build_indexes(&db).await?;
                    tracing::info!("Indexes built successfully.");
                }
                Subcommands::Migrate => {
                    tracing::info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
                    let db = chronicle::db::MongoDb::connect(&config.mongodb).await?;
                    crate::migrations::migrate(&db).await?;
                    tracing::info!("Migration completed successfully.");
                }
                _ => (),
            }
        }
        Ok(PostCommand::Exit)
    }
}

#[derive(Debug, Subcommand)]
pub enum Subcommands {
    /// Generate a JWT token using the available config.
    #[cfg(feature = "api")]
    GenerateJWT(api::GenerateJWTCommand),
    /// Fill analytics from an input source.
    #[cfg(feature = "analytics")]
    FillAnalytics(analytics::FillAnalyticsCommand),
    /// Fill interval analytics from Chronicle's database.
    #[cfg(feature = "analytics")]
    FillIntervalAnalytics(analytics::FillIntervalAnalyticsCommand),
    /// Clear the Chronicle database.
    #[cfg(debug_assertions)]
    ClearDatabase {
        /// Run the application after this command.
        #[arg(short, long)]
        run: bool,
    },
    /// Manually build indexes.
    BuildIndexes,
    /// Migrate to a new version.
    Migrate,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PostCommand {
    Start,
    Exit,
}
