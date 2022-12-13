// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::{ChronicleConfig, ConfigError};

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
#[command(author, version, about, next_display_order = None)]
pub struct ClArgs {
    /// The location of the configuration file.
    #[arg(short, long)]
    pub config: Option<String>,
    /// MongoDb arguments.
    #[command(flatten, next_help_heading = "MongoDb")]
    pub mongodb: MongoDbArgs,
    /// INX arguments.
    #[cfg(feature = "inx")]
    #[command(flatten, next_help_heading = "INX")]
    pub inx: InxArgs,
    /// Rest API arguments.
    #[cfg(feature = "api")]
    #[command(flatten, next_help_heading = "API")]
    pub api: ApiArgs,
    /// InfluxDb arguments.
    #[cfg(any(feature = "analytics", feature = "metrics"))]
    #[command(flatten, next_help_heading = "InfluxDb")]
    pub influxdb: InfluxDbArgs,
    /// Loki arguments.
    #[cfg(feature = "loki")]
    #[command(flatten, next_help_heading = "Loki")]
    pub loki: LokiArgs,
    /// Subcommands.
    #[command(subcommand)]
    pub subcommand: Option<Subcommands>,
}

#[derive(Args, Debug)]
pub struct MongoDbArgs {
    /// The MongoDb connection string.
    #[arg(long, value_name = "CONN_STR", env = "MONGODB_CONN_STR", default_value = "mongodb://localhost:27017")]
    pub mongodb_conn_str: Option<String>,
    /// The MongoDb username. 
    #[arg(long, value_name = "USERNAME", env = "MONGODB_USERNAME", default_value = "root")]
    pub mongodb_username: Option<String>,
    /// The MongoDb password. 
    #[arg(long, value_name = "PASSWORD", env = "MONGODB_PASSWORD", default_value = "root")]
    pub mongodb_password: Option<String>,
    /// The main database name.
    #[arg(long, value_name = "NAME", default_value = "chronicle")]
    pub mongodb_database_name: Option<String>,
    /// The MongoDb minimum pool size.
    #[arg(long, value_name = "SIZE", default_value = "2")]
    pub mongodb_min_pool_size: Option<usize>,
}

#[cfg(feature = "inx")]
#[derive(Args, Debug)]
pub struct InxArgs {
    /// Toggles the INX synchronization workflow.
    #[arg(long, default_value = "true")]
    pub inx_enabled: Option<bool>,
    /// The address of the node INX interface Chronicle tries to connect to - if enabled.
    #[arg(long, default_value = "http://localhost:9029")]
    pub inx_url: Option<String>,
    /// Milestone at which synchronization should begin. If set to `1` Chronicle will try to sync back until the
    /// genesis block. If set to `0` Chronicle will start syncing from the most recent milestone it received.
    #[arg(long, default_value = "0")]
    pub inx_sync_start: Option<u32>,
    /// Time to wait until a new connection attempt is made.
    #[arg(long, default_value = "5s")]
    pub inx_retry_interval: Option<String>,
    /// Maximum number of tries to establish an INX connection.
    #[arg(long, default_value = "30")]
    pub inx_retry_count: Option<usize>,
}

#[cfg(feature = "api")]
#[derive(Args, Debug)]
pub struct ApiArgs {
    /// Toggle REST API.
    #[arg(long, default_value = "true")]
    pub api_enabled: Option<bool>,
    /// API listening port.
    #[arg(long, default_value = "8042")]
    pub api_port: Option<u16>,
    /// CORS setting.
    #[arg(long, default_value = "0.0.0.0")]
    pub api_allow_origins: Option<String>,
    /// Public API routes.
    #[arg(long = "public-route", value_name = "ROUTE", default_value = "api/core/v2/*")]
    pub public_routes: Vec<String>,
    /// Maximum nubmer of results returned by a single API call.
    #[arg(long, default_value = "1000")]
    pub max_page_size: Option<usize>,
    /// JWT arguments.
    #[command(flatten)]
    pub jwt: JwtArgs,
}

#[derive(Args, Debug)]
pub struct JwtArgs {
    /// The location of the identity file for JWT auth.
    #[arg(long, env = "JWT_IDENTITY", default_value = None)]
    pub jwt_identity: Option<String>,
    /// The password used for JWT authentication.
    #[arg(long, env = "JWT_PASSWORD", default_value = "password")]
    pub jwt_password: Option<String>,
    // The salt used for JWT authentication.
    #[arg(long, env = "JWT_SALT", default_value = "saltines")]
    pub jwt_salt: Option<String>,
    /// The setting for when the (JWT) token expires.
    #[arg(long, default_value = "72h")]
    pub jwt_expiration: Option<String>,
}

#[cfg(any(feature = "analytics", feature = "metrics"))]
#[derive(Args, Debug)]
pub struct InfluxDbArgs {
    /// The url pointing to an InfluxDb instance.
    #[arg(long, default_value = "http://localhost:8086")]
    pub influxdb_url: Option<String>,
    /// The InfluxDb username. 
    #[arg(long, env = "INFLUXDB_USERNAME", default_value = "root")]
    pub influxdb_username: Option<String>,
    /// The InfluxDb password. 
    #[arg(long, env = "INFLUXDB_PASSWORD", default_value = "password")]
    pub influxdb_password: Option<String>,
    /// Toggle InfluxDb time-series analytics writes.
    #[arg(long, default_value = "true")]
    pub analytics_enabled: Option<bool>,
    /// Toggle InfluxDb time-series metrics writes.
    #[arg(long, default_value = "true")]
    pub metrics_enabled: Option<bool>,
    /// The Analytics database name.
    #[arg(long, default_value = "chronicle_analytics")]
    pub analytics_database_name: Option<String>,
    /// The Metrics database name.
    #[arg(long, default_value = "chronicle_metrics")]
    pub metrics_database_name: Option<String>,
}

#[cfg(feature = "loki")]
#[derive(Args, Debug)]
pub struct LokiArgs {
    /// Toggle Grafana Loki log writes.
    #[arg(long, default_value = "true")]
    pub loki_enabled: Option<bool>,
    /// The url pointing to a Grafana Loki instance.
    #[arg(long, default_value = "http://localhost:3100")]
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

        #[cfg(all(feature = "stardust", feature = "inx"))]
        {
            if let Some(connect_url) = &self.inx.inx_url {
                config.inx.connect_url = connect_url.clone();
            }
            if let Some(enabled) = self.inx.inx_enabled {
                config.inx.enabled = enabled;
            }
            if let Some(sync_start) = self.inx.inx_sync_start {
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
            if let Some(password) = &self.api.jwt.jwt_password {
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
            if let Some(path) = &self.api.jwt.jwt_identity {
                config.api.identity_path.replace(path.clone());
            }
            config.api.enabled = self.api.api_enabled.unwrap();
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
                    let start_milestone = if let Some(index) = start_milestone {
                        *index
                    } else {
                        db.collection::<chronicle::db::collections::MilestoneCollection>()
                            .get_oldest_milestone()
                            .await?
                            .map(|ts| ts.milestone_index)
                            .unwrap_or_default()
                    };
                    let end_milestone = if let Some(index) = end_milestone {
                        *index
                    } else {
                        db.collection::<chronicle::db::collections::MilestoneCollection>()
                            .get_newest_milestone()
                            .await?
                            .map(|ts| ts.milestone_index)
                            .unwrap_or_default()
                    };
                    let influx_db = chronicle::db::influxdb::InfluxDb::connect(&config.influxdb).await?;

                    let num_tasks = num_tasks.unwrap_or(1);
                    let mut join_set = tokio::task::JoinSet::new();
                    for i in 0..num_tasks {
                        let db = db.clone();
                        let influx_db = influx_db.clone();
                        let analytics_choice = analytics.clone();
                        join_set.spawn(async move {
                            let mut selected_analytics = if analytics_choice.is_empty() {
                                chronicle::db::collections::analytics::all_analytics()
                            } else {
                                let mut tmp: std::collections::HashSet<AnalyticsChoice> =
                                    analytics_choice.iter().copied().collect();
                                tmp.drain().map(Into::into).collect()
                            };

                            for index in (*start_milestone..*end_milestone).skip(i).step_by(num_tasks) {
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
                            }
                            eyre::Result::<_>::Ok(())
                        });
                    }
                    while let Some(res) = join_set.join_next().await {
                        // Panic: Acceptable risk
                        res.unwrap()?;
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum AnalyticsChoice {
    AddressActivity,
    Addresses,
    BaseToken,
    LedgerOutputs,
    OutputActivity,
    LedgerSize,
    UnclaimedTokens,
    BlockActivity,
    UnlockConditions,
    ProtocolParameters,
}

#[cfg(all(feature = "analytics", feature = "stardust"))]
impl From<AnalyticsChoice> for Box<dyn chronicle::db::collections::analytics::Analytic> {
    fn from(value: AnalyticsChoice) -> Self {
        use chronicle::db::collections::analytics::{
            AddressActivityAnalytics, AddressAnalytics, BaseTokenActivityAnalytics, BlockActivityAnalytics,
            LedgerOutputAnalytics, LedgerSizeAnalytics, OutputActivityAnalytics, ProtocolParametersAnalytics,
            UnclaimedTokenAnalytics, UnlockConditionAnalytics,
        };

        match value {
            AnalyticsChoice::AddressActivity => Box::new(AddressActivityAnalytics),
            AnalyticsChoice::Addresses => Box::new(AddressAnalytics),
            AnalyticsChoice::BaseToken => Box::new(BaseTokenActivityAnalytics),
            AnalyticsChoice::LedgerOutputs => Box::new(LedgerOutputAnalytics),
            AnalyticsChoice::OutputActivity => Box::new(OutputActivityAnalytics),
            AnalyticsChoice::LedgerSize => Box::new(LedgerSizeAnalytics),
            AnalyticsChoice::UnclaimedTokens => Box::new(UnclaimedTokenAnalytics),
            AnalyticsChoice::BlockActivity => Box::new(BlockActivityAnalytics),
            AnalyticsChoice::UnlockConditions => Box::new(UnlockConditionAnalytics),
            AnalyticsChoice::ProtocolParameters => Box::new(ProtocolParametersAnalytics),
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
