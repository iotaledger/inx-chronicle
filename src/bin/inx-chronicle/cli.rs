// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::{ChronicleConfig, ConfigError};

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
#[command(author, version, about, next_display_order = None)]
pub struct ClArgs {
    /// The location of the configuration file.
    #[arg(short, long, env = "CONFIG_PATH")]
    pub config: Option<String>,
    /// Rest API arguments.
    #[cfg(feature = "api")]
    #[command(flatten)]
    pub api: ApiArgs,
    /// InfluxDb arguments.
    #[cfg(any(feature = "analytics", feature = "metrics"))]
    #[command(flatten)]
    pub influxdb: InfluxDbArgs,
    /// INX arguments.
    #[cfg(feature = "inx")]
    #[command(flatten)]
    pub inx: InxArgs,
    /// MongoDb arguments.
    #[command(flatten)]
    pub mongodb: MongoDbArgs,
    /// Loki arguments.
    #[cfg(feature = "loki")]
    #[command(flatten)]
    pub loki: LokiArgs,
    /// Subcommands.
    #[command(subcommand)]
    pub subcommand: Option<Subcommands>,
}

#[cfg(feature = "api")]
#[derive(Args, Debug)]
pub struct ApiArgs {
    /// Toggle REST API.
    #[arg(long, env = "REST_API_ENABLED")]
    pub api_enabled: Option<bool>,
    /// JWT arguments.
    #[command(flatten)]
    pub jwt: JwtArgs,
}

#[derive(Args, Debug)]
pub struct JwtArgs {
    /// The location of the identity file for JWT auth.
    #[arg(long = "api-jwt-identity", env = "JWT_IDENTITY_PATH")]
    pub identity_path: Option<String>,
    /// The password used for JWT authentication.
    #[arg(long = "api-jwt-password")]
    pub password: Option<String>,
}

#[cfg(feature = "inx")]
#[derive(Args, Debug)]
pub struct InxArgs {
    /// Toggle INX write workflow.
    #[arg(long, env = "INX_ENABLED")]
    pub inx_enabled: Option<bool>,
    /// The address of the INX interface provided by the node.
    #[arg(long, env = "INX_URL")]
    pub inx_url: Option<String>,
    /// Milestone at which synchronization should begin. A value of `1` means syncing back until genesis (default).
    #[arg(long = "inx-sync-start")]
    pub sync_start: Option<u32>,
}

#[derive(Args, Debug)]
pub struct MongoDbArgs {
    /// The MongoDB connection string.
    #[arg(long, env = "MONGODB_CONN_STR")]
    pub mongodb_conn_str: Option<String>,
    /// The MongoDB database.
    #[arg(long, env = "MONGODB_DATABASE")]
    pub mongodb_database: Option<String>,
}

#[cfg(any(feature = "analytics", feature = "metrics"))]
#[derive(Args, Debug)]
pub struct InfluxDbArgs {
    /// Toggle InfluxDb time-series metrics writes.
    #[arg(long, env = "METRICS_ENABLED")]
    pub metrics_enabled: Option<bool>,
    /// Toggle InfluxDb time-series analytics writes.
    #[arg(long, env = "ANALYTICS_ENABLED")]
    pub analytics_enabled: Option<bool>,
    /// The url pointing to an InfluxDb instance.
    #[arg(long, env = "INFLUXDB_URL")]
    pub influxdb_url: Option<String>,
}

#[cfg(feature = "loki")]
#[derive(Args, Debug)]
pub struct LokiArgs {
    /// Toggle Grafana Loki log writes.
    #[arg(long, env = "LOKI_ENABLED")]
    pub loki_enabled: Option<bool>,
    /// The url pointing to a Grafana Loki instance.
    #[arg(long, env = "LOKI_URL")]
    pub loki_url: Option<String>,
}

impl ClArgs {
    /// Get a config file with CLI args applied.
    pub fn get_config(&self) -> Result<ChronicleConfig, ConfigError> {
        let mut config = self
            .config
            .as_ref()
            .map(ChronicleConfig::from_file)
            .transpose()?
            .unwrap_or_default();

        if let Some(conn_str) = &self.mongodb.mongodb_conn_str {
            config.mongodb.conn_str = conn_str.clone();
        }

        if let Some(db_name) = &self.mongodb.mongodb_database {
            config.mongodb.database_name = db_name.clone();
        }

        #[cfg(all(feature = "stardust", feature = "inx"))]
        {
            if let Some(connect_url) = &self.inx.inx_url {
                config.inx.connect_url = connect_url.clone();
            }
            if let Some(enabled) = self.inx.inx_enabled {
                config.inx.enabled = enabled;
            }
            if let Some(sync_start) = self.inx.sync_start {
                config.inx.sync_start_milestone = sync_start.into();
            }
        }

        #[cfg(feature = "analytics")]
        {
            if let Some(enabled) = self.influxdb.analytics_enabled {
                config.influxdb.analytics_enabled = enabled;
            }
        }

        #[cfg(feature = "metrics")]
        {
            if let Some(enabled) = self.influxdb.metrics_enabled {
                config.influxdb.metrics_enabled = enabled;
            }
        }

        #[cfg(any(feature = "analytics", feature = "metrics"))]
        {
            if let Some(url) = &self.influxdb.influxdb_url {
                config.influxdb.url = url.clone();
            }
        }

        #[cfg(feature = "api")]
        {
            if let Some(password) = &self.api.jwt.password {
                config.api.password_hash = hex::encode(
                    argon2::hash_raw(
                        password.as_bytes(),
                        config.api.password_salt.as_bytes(),
                        &Into::into(&config.api.argon_config),
                    )
                    // TODO: Replace this once we switch to a better error lib
                    .expect("invalid JWT config"),
                );
            }
            if let Some(path) = &self.api.jwt.identity_path {
                config.api.identity_path.replace(path.clone());
            }
            if let Some(enabled) = self.api.api_enabled {
                config.api.enabled = enabled;
            }
        }

        #[cfg(feature = "loki")]
        {
            if let Some(connect_url) = &self.loki.loki_url {
                config.loki.connect_url = connect_url.clone();
            }
            if let Some(enabled) = self.loki.loki_enabled {
                config.loki.enabled = enabled;
            }
        }

        Ok(config)
    }

    /// Process subcommands and return whether the app should early exit.
    #[allow(unused)]
    #[allow(clippy::collapsible_match)]
    pub async fn process_subcommands(&self, config: &ChronicleConfig) -> eyre::Result<PostCommand> {
        if let Some(subcommand) = &self.subcommand {
            match subcommand {
                #[cfg(feature = "api")]
                Subcommands::GenerateJWT => {
                    use crate::api::ApiData;
                    let api_data = ApiData::try_from(config.api.clone()).expect("invalid API config");
                    let claims = auth_helper::jwt::Claims::new(
                        ApiData::ISSUER,
                        uuid::Uuid::new_v4().to_string(),
                        ApiData::AUDIENCE,
                    )
                    .unwrap() // Panic: Cannot fail.
                    .expires_after_duration(api_data.jwt_expiration)
                    .map_err(crate::api::AuthError::InvalidJwt)?;
                    let exp_ts = time::OffsetDateTime::from_unix_timestamp(claims.exp.unwrap() as _).unwrap();
                    let jwt = auth_helper::jwt::JsonWebToken::new(claims, api_data.secret_key.as_ref())
                        .map_err(crate::api::AuthError::InvalidJwt)?;
                    tracing::info!("Bearer {}", jwt);
                    tracing::info!(
                        "Expires: {} ({})",
                        exp_ts,
                        humantime::format_duration(api_data.jwt_expiration)
                    );
                    return Ok(PostCommand::Exit);
                }
                #[cfg(all(feature = "analytics", feature = "stardust"))]
                Subcommands::FillAnalytics {
                    start_milestone,
                    end_milestone,
                    num_tasks,
                    analytics,
                } => {
                    tracing::info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
                    let db = chronicle::db::MongoDb::connect(&config.mongodb).await?;
                    let mut start_milestone = if let Some(index) = start_milestone {
                        **index
                    } else {
                        *db.collection::<chronicle::db::collections::MilestoneCollection>()
                            .get_oldest_milestone()
                            .await?
                            .map(|ts| ts.milestone_index)
                            .unwrap_or_default()
                    };
                    let mut end_milestone = if let Some(index) = end_milestone {
                        **index
                    } else {
                        *db.collection::<chronicle::db::collections::MilestoneCollection>()
                            .get_newest_milestone()
                            .await?
                            .map(|ts| ts.milestone_index)
                            .unwrap_or_default()
                    };
                    let influx_db = chronicle::db::influxdb::InfluxDb::connect(&config.influxdb).await?;

                    let num_tasks = num_tasks.unwrap_or(1);
                    let mut tasks = tokio::task::JoinSet::<eyre::Result<()>>::new();
                    // Deterministic task cancellation
                    static CANCEL: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
                    // Inclusive end
                    let total_milestones = end_milestone - start_milestone + 1;
                    for i in 0..num_tasks {
                        let db = db.clone();
                        let influx_db = influx_db.clone();
                        let analytics_choice = analytics.clone();
                        // Each task gets an even share of the milestones, but we must also
                        // assign the remainders if the total number of milestones isn't evenly
                        // divided.
                        let task_milestones = (total_milestones / num_tasks as u32)
                            + ((i as u32) < (total_milestones % num_tasks as u32)) as u32;
                        // Account for inclusive end (again)
                        end_milestone = start_milestone + task_milestones - 1;

                        tasks.spawn(fill_analytics(
                            analytics_choice,
                            db,
                            influx_db,
                            start_milestone,
                            end_milestone,
                            &CANCEL,
                        ));

                        // Account for inclusive end
                        start_milestone = end_milestone + 1;
                    }
                    tokio::select! {
                        _ = shutdown() => {
                            CANCEL.store(true, std::sync::atomic::Ordering::Relaxed);
                            join_all(&mut tasks).await?
                        },
                        res = join_all(&mut tasks) => res?,
                    }
                    return Ok(PostCommand::Exit);
                }
                #[cfg(debug_assertions)]
                Subcommands::ClearDatabase { run } => {
                    tracing::info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
                    let db = chronicle::db::MongoDb::connect(&config.mongodb).await?;
                    db.clear().await?;
                    tracing::info!("Database cleared successfully.");
                    if !run {
                        return Ok(PostCommand::Exit);
                    }
                }
                #[cfg(feature = "stardust")]
                Subcommands::BuildIndexes => {
                    tracing::info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
                    let db = chronicle::db::MongoDb::connect(&config.mongodb).await?;
                    super::build_indexes(&db).await?;
                    tracing::info!("Indexes built successfully.");
                    return Ok(PostCommand::Exit);
                }
                _ => (),
            }
        }
        Ok(PostCommand::Start)
    }
}

#[cfg(all(feature = "analytics", feature = "stardust"))]
async fn shutdown() {
    crate::process::interrupt_or_terminate().await;
    tracing::info!("received ctrl-c or terminate");
}

#[cfg(all(feature = "analytics", feature = "stardust"))]
async fn join_all(tasks: &mut tokio::task::JoinSet<eyre::Result<()>>) -> eyre::Result<()> {
    while let Some(res) = tasks.join_next().await {
        // Panic: Acceptable risk
        res.unwrap()?;
    }
    Ok(())
}

#[cfg(all(feature = "analytics", feature = "stardust"))]
async fn fill_analytics(
    analytics_choice: Vec<AnalyticsChoice>,
    db: chronicle::db::MongoDb,
    influx_db: chronicle::db::influxdb::InfluxDb,
    start_milestone: u32,
    end_milestone: u32,
    cancel: &std::sync::atomic::AtomicBool,
) -> eyre::Result<()> {
    let mut selected_analytics = if analytics_choice.is_empty() {
        chronicle::db::collections::analytics::all_analytics()
    } else {
        let mut tmp: std::collections::HashSet<AnalyticsChoice> = analytics_choice.iter().copied().collect();
        tmp.drain().map(Into::into).collect()
    };

    tracing::info!("Computing the following analytics: {:?}", selected_analytics);

    for index in start_milestone..=end_milestone {
        let milestone_index = index.into();
        if let Some(milestone_timestamp) = db
            .collection::<chronicle::db::collections::MilestoneCollection>()
            .get_milestone_timestamp(milestone_index)
            .await?
        {
            #[cfg(feature = "metrics")]
            let start_time = std::time::Instant::now();

            super::stardust_inx::gather_analytics(
                &db,
                &influx_db,
                &mut selected_analytics,
                milestone_index,
                milestone_timestamp,
            )
            .await?;

            #[cfg(feature = "metrics")]
            {
                let elapsed = start_time.elapsed();
                influx_db
                    .metrics()
                    .insert(chronicle::db::collections::metrics::AnalyticsMetrics {
                        time: chrono::Utc::now(),
                        milestone_index,
                        analytics_time: elapsed.as_millis() as u64,
                        chronicle_version: std::env!("CARGO_PKG_VERSION").to_string(),
                    })
                    .await?;
            }
            tracing::info!("Finished analytics for milestone {}", milestone_index);
        } else {
            tracing::info!("No milestone in database for index {}", milestone_index);
        }
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
    }
    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum AnalyticsChoice {
    // Please keep the alphabetic order.
    Addresses,
    BaseToken,
    BlockActivity,
    DailyActiveAddresses,
    LedgerOutputs,
    LedgerSize,
    OutputActivity,
    ProtocolParameters,
    UnclaimedTokens,
    UnlockConditions,
}

#[cfg(all(feature = "analytics", feature = "stardust"))]
impl From<AnalyticsChoice> for Box<dyn chronicle::db::collections::analytics::Analytic> {
    fn from(value: AnalyticsChoice) -> Self {
        use chronicle::db::collections::analytics::{
            AddressAnalytics, BaseTokenActivityAnalytics, BlockActivityAnalytics, DailyActiveAddressesAnalytics,
            LedgerOutputAnalytics, LedgerSizeAnalytics, OutputActivityAnalytics, ProtocolParametersAnalytics,
            UnclaimedTokenAnalytics, UnlockConditionAnalytics,
        };

        match value {
            // Please keep the alphabetic order.
            AnalyticsChoice::Addresses => Box::new(AddressAnalytics),
            AnalyticsChoice::BaseToken => Box::new(BaseTokenActivityAnalytics),
            AnalyticsChoice::BlockActivity => Box::new(BlockActivityAnalytics),
            AnalyticsChoice::DailyActiveAddresses => Box::<DailyActiveAddressesAnalytics>::default(),
            AnalyticsChoice::LedgerOutputs => Box::<LedgerOutputAnalytics>::default(),
            AnalyticsChoice::LedgerSize => Box::<LedgerSizeAnalytics>::default(),
            AnalyticsChoice::OutputActivity => Box::new(OutputActivityAnalytics),
            AnalyticsChoice::ProtocolParameters => Box::new(ProtocolParametersAnalytics),
            AnalyticsChoice::UnclaimedTokens => Box::<UnclaimedTokenAnalytics>::default(),
            AnalyticsChoice::UnlockConditions => Box::<UnlockConditionAnalytics>::default(),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Subcommands {
    /// Generate a JWT token using the available config.
    #[cfg(feature = "api")]
    GenerateJWT,
    #[cfg(all(feature = "analytics", feature = "stardust"))]
    FillAnalytics {
        /// The inclusive starting milestone index.
        #[arg(short, long)]
        start_milestone: Option<chronicle::types::tangle::MilestoneIndex>,
        /// The exclusive ending milestone index.
        #[arg(short, long)]
        end_milestone: Option<chronicle::types::tangle::MilestoneIndex>,
        /// The number of parallel tasks to use when filling the analytics.
        #[arg(short, long)]
        num_tasks: Option<usize>,
        /// Select a subset of analytics to compute.
        #[arg(long)]
        analytics: Vec<AnalyticsChoice>,
    },
    /// Clear the chronicle database.
    #[cfg(debug_assertions)]
    ClearDatabase {
        /// Run the application after this command.
        #[arg(short, long)]
        run: bool,
    },
    /// Manually build indexes.
    #[cfg(feature = "stardust")]
    BuildIndexes,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PostCommand {
    Start,
    Exit,
}
