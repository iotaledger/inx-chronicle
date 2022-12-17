// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::mongodb::config as mongodb;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::{ChronicleConfig, ConfigError};

macro_rules! to_str {
    ($arg:expr) => {
        const_format::formatcp!("{}", $arg)
    };
}

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
// #[command(author, version, about, next_display_order = None)]
#[command(author, version, about)]
pub struct ClArgs {
    /// The location of the configuration file.
    #[arg(short, long, value_name = "FILEPATH")]
    pub config: Option<String>,
    /// MongoDb arguments.
    #[command(flatten, next_help_heading = "MongoDb")]
    pub mongodb: MongoDbArgs,
    /// InfluxDb arguments.
    #[cfg(any(feature = "analytics", feature = "metrics"))]
    #[command(flatten, next_help_heading = "InfluxDb")]
    pub influxdb: InfluxDbArgs,
    /// INX arguments.
    #[cfg(feature = "inx")]
    #[command(flatten, next_help_heading = "INX")]
    pub inx: InxArgs,
    /// Rest API arguments.
    #[cfg(feature = "api")]
    #[command(flatten, next_help_heading = "API")]
    pub api: ApiArgs,
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
    #[arg(
        long,
        value_name = "CONN_STR",
        env = "MONGODB_CONN_STR",
        default_value = mongodb::DEFAULT_CONN_STR,
    )]
    pub mongodb_conn_str: Option<String>,
    /// The MongoDb username.
    #[arg(long, value_name = "USERNAME", env = "MONGODB_USERNAME", default_value = mongodb::DEFAULT_USERNAME)]
    pub mongodb_username: Option<String>,
    /// The MongoDb password.
    #[arg(long, value_name = "PASSWORD", env = "MONGODB_PASSWORD", default_value = mongodb::DEFAULT_PASSWORD)]
    pub mongodb_password: Option<String>,
    /// The MongoDb database name.
    #[arg(long, value_name = "NAME", default_value = mongodb::DEFAULT_DATABASE_NAME)]
    pub mongodb_database_name: Option<String>,
    /// The MongoDb minimum pool size.
    #[arg(long, value_name = "SIZE", default_value = to_str!(mongodb::DEFAULT_MIN_POOL_SIZE))]
    pub mongodb_min_pool_size: Option<u32>,
}

#[cfg(any(feature = "analytics", feature = "metrics"))]
use chronicle::db::influxdb::config as influxdb;

#[cfg(any(feature = "analytics", feature = "metrics"))]
#[derive(Args, Debug)]
pub struct InfluxDbArgs {
    /// The url pointing to an InfluxDb instance.
    #[arg(long, value_name = "URL", default_value = influxdb::DEFAULT_URL)]
    pub influxdb_url: Option<String>,
    /// The InfluxDb username.
    #[arg(long, value_name = "USERNAME", env = "INFLUXDB_USERNAME", default_value = influxdb::DEFAULT_USERNAME)]
    pub influxdb_username: Option<String>,
    /// The InfluxDb password.
    #[arg(long, value_name = "PASSWORD", env = "INFLUXDB_PASSWORD", default_value = influxdb::DEFAULT_PASSWORD)]
    pub influxdb_password: Option<String>,
    /// Toggle InfluxDb time-series analytics writes.
    #[cfg(feature = "analytics")]
    #[arg(long, value_name = "BOOL", default_value = to_str!(influxdb::DEFAULT_ANALYTICS_ENABLED))]
    pub analytics_enabled: Option<bool>,
    /// The Analytics database name.
    #[cfg(feature = "analytics")]
    #[arg(long, value_name = "NAME", default_value = influxdb::DEFAULT_ANALYTICS_DATABASE_NAME)]
    pub analytics_database_name: Option<String>,
    /// Toggle InfluxDb time-series metrics writes.
    #[cfg(feature = "metrics")]
    #[arg(long, value_name = "BOOL", default_value = to_str!(influxdb::DEFAULT_METRICS_ENABLED))]
    pub metrics_enabled: Option<bool>,
    /// The Metrics database name.
    #[cfg(feature = "metrics")]
    #[arg(long, value_name = "NAME", default_value = influxdb::DEFAULT_METRICS_DATABASE_NAME)]
    pub metrics_database_name: Option<String>,
}

#[cfg(feature = "inx")]
use crate::stardust_inx::config as inx;

#[cfg(feature = "inx")]
#[derive(Args, Debug)]
pub struct InxArgs {
    /// Toggles the INX synchronization workflow.
    #[arg(long, value_name = "BOOL", default_value = to_str!(inx::DEFAULT_ENABLED))]
    pub inx_enabled: Option<bool>,
    /// The address of the node INX interface Chronicle tries to connect to - if enabled.
    #[arg(long, value_name = "URL", default_value = inx::DEFAULT_URL)]
    pub inx_url: Option<String>,
    /// Time to wait until a new connection attempt is made.
    #[arg(long, value_name = "DURATION", value_parser = parse_duration, default_value = inx::DEFAULT_RETRY_INTERVAL)]
    pub inx_retry_interval: Option<std::time::Duration>,
    /// Maximum number of tries to establish an INX connection.
    #[arg(long, value_name = "COUNT", default_value = to_str!(inx::DEFAULT_RETRY_COUNT))]
    pub inx_retry_count: Option<usize>,
    /// Milestone at which synchronization should begin. If set to `1` Chronicle will try to sync back until the
    /// genesis block. If set to `0` Chronicle will start syncing from the most recent milestone it received.
    #[arg(long, value_name = "START", default_value = to_str!(inx::DEFAULT_SYNC_START))]
    pub inx_sync_start: Option<u32>,
}

#[cfg(feature = "api")]
use crate::api::config as api;

#[cfg(feature = "api")]
#[derive(Args, Debug)]
pub struct ApiArgs {
    /// Toggle REST API.
    #[arg(long, value_name = "BOOL", default_value = to_str!(api::DEFAULT_ENABLED))]
    pub api_enabled: Option<bool>,
    /// API listening port.
    #[arg(long, value_name = "PORT", default_value = to_str!(api::DEFAULT_PORT))]
    pub api_port: Option<u16>,
    /// CORS setting.
    #[arg(long = "allow-origin", value_name = "IP", default_value = api::DEFAULT_ALLOW_ORIGINS)]
    pub allow_origins: Vec<String>,
    /// Public API routes.
    #[arg(long = "public-route", value_name = "ROUTE", default_value = api::DEFAULT_PUBLIC_ROUTES)]
    pub public_routes: Vec<String>,
    /// Maximum number of results returned by a single API call.
    #[arg(long, value_name = "SIZE", default_value = to_str!(api::DEFAULT_MAX_PAGE_SIZE))]
    pub max_page_size: Option<usize>,
    /// JWT arguments.
    #[command(flatten)]
    pub jwt: JwtArgs,
}

#[cfg(feature = "api")]
#[derive(Args, Debug)]
pub struct JwtArgs {
    /// The location of the identity file for JWT auth.
    #[arg(long, value_name = "FILEPATH", env = "JWT_IDENTITY", default_value = None)]
    pub jwt_identity: Option<String>,
    /// The password used for JWT authentication.
    #[arg(long, value_name = "PASSWORD", env = "JWT_PASSWORD", default_value = api::DEFAULT_JWT_PASSWORD)]
    pub jwt_password: Option<String>,
    /// The salt used for JWT authentication.
    #[arg(long, value_name = "SALT", env = "JWT_SALT", default_value = api::DEFAULT_JWT_SALT)]
    pub jwt_salt: Option<String>,
    /// The setting for when the (JWT) token expires.
    #[arg(long, value_name = "DURATION", value_parser = parse_duration, default_value = api::DEFAULT_JWT_EXPIRATION)]
    pub jwt_expiration: Option<std::time::Duration>,
}

#[cfg(feature = "loki")]
#[derive(Args, Debug)]
pub struct LokiArgs {
    /// Toggle Grafana Loki log writes.
    #[arg(long, value_name = "BOOL", default_value = to_str!(crate::config::loki::DEFAULT_LOKI_ENABLED))]
    pub loki_enabled: Option<bool>,
    /// The url pointing to a Grafana Loki instance.
    #[arg(long, value_name = "URL", default_value = crate::config::loki::DEFAULT_LOKI_URL)]
    pub loki_url: Option<String>,
}

fn parse_duration(arg: &str) -> Result<std::time::Duration, humantime::DurationError> {
    arg.parse::<humantime::Duration>().map(Into::into)
}

impl ClArgs {
    /// Get a config from a file (specified via the `--config` option) or from provided CLI args combined
    /// with defaults for those that are not provided. Note that a config file must be fully specified
    /// as it cannot be overwritten with the CLI defaults. If you plan on using a `config.toml` use
    /// Chronicle's `create-config' tool to make sure of that.
    pub fn get_config(&self) -> Result<ChronicleConfig, ConfigError> {
        if let Some(config_path) = &self.config {
            return ChronicleConfig::from_file(config_path);
        }

        let mut config = ChronicleConfig::default();

        // MongoDb
        // Note: all unwraps are fine because we defined defaults for all, so none of them can be None ;)
        config.mongodb.conn_str = self.mongodb.mongodb_conn_str.as_ref().unwrap().clone();
        config.mongodb.database_name = self.mongodb.mongodb_database_name.as_ref().unwrap().clone();
        config.mongodb.username = self.mongodb.mongodb_username.as_ref().unwrap().clone();
        config.mongodb.password = self.mongodb.mongodb_password.as_ref().unwrap().clone();
        config.mongodb.min_pool_size = self.mongodb.mongodb_min_pool_size.unwrap();

        // INX
        #[cfg(all(feature = "stardust", feature = "inx"))]
        {
            config.inx.enabled = self.inx.inx_enabled.unwrap();
            config.inx.url = self.inx.inx_url.as_ref().unwrap().clone();
            config.inx.conn_retry_interval = self.inx.inx_retry_interval.unwrap();
            config.inx.conn_retry_count = self.inx.inx_retry_count.unwrap();
            config.inx.sync_start_milestone = self.inx.inx_sync_start.unwrap().into();
        }

        // InfluxDb
        #[cfg(any(feature = "analytics", feature = "metrics"))]
        {
            config.influxdb.url = self.influxdb.influxdb_url.as_ref().unwrap().clone();
            config.influxdb.username = self.influxdb.influxdb_username.as_ref().unwrap().clone();
            config.influxdb.password = self.influxdb.influxdb_password.as_ref().unwrap().clone();
        }
        #[cfg(feature = "analytics")]
        {
            config.influxdb.analytics_enabled = self.influxdb.analytics_enabled.unwrap();
            config.influxdb.analytics_database_name = self.influxdb.analytics_database_name.as_ref().unwrap().clone();
        }
        #[cfg(feature = "metrics")]
        {
            config.influxdb.metrics_enabled = self.influxdb.metrics_enabled.unwrap();
            config.influxdb.metrics_database_name = self.influxdb.metrics_database_name.as_ref().unwrap().clone();
        }

        // API
        #[cfg(feature = "api")]
        {
            config.api.enabled = self.api.api_enabled.unwrap();
            config.api.port = self.api.api_port.unwrap();
            config.api.allow_origins = (&self.api.allow_origins).into();
            config.api.jwt_password = self.api.jwt.jwt_password.as_ref().unwrap().clone();
            config.api.jwt_salt = self.api.jwt.jwt_salt.as_ref().unwrap().clone();
            config.api.jwt_identity_file = self.api.jwt.jwt_identity.clone();
            config.api.max_page_size = self.api.max_page_size.unwrap();
            config.api.public_routes = self.api.public_routes.clone();
        }

        // Loki
        #[cfg(feature = "loki")]
        {
            config.loki.url = self.loki.loki_url.as_ref().unwrap().clone();
            config.loki.enabled = self.loki.loki_enabled.unwrap();
        }

        Ok(config)
    }

    /// Process subcommands and return whether the app should early exit.
    #[allow(unused)]
    #[allow(clippy::collapsible_match)]
    pub async fn process_subcommands(&self, config: &ChronicleConfig) -> eyre::Result<PostCommand> {
        if let Some(subcommand) = &self.subcommand {
            match subcommand {
                Subcommands::CreateConfig { file_path } => {
                    let toml_config = format!(
                        "# This file was auto-generated by Chronicle's `create-config` tool at version '{}'.\n# Check CHANGELOG.md for breaking configuration changes, and re-run the tool if necessary.\n\n{}",
                        std::env::var("CARGO_PKG_VERSION").unwrap(),
                        toml::to_string_pretty(config)?
                    );
                    std::fs::write(file_path.as_ref().unwrap(), toml_config)?;
                    tracing::info!("Written generated config to: '{}'", file_path.as_ref().unwrap());
                    return Ok(PostCommand::Exit);
                }
                #[cfg(feature = "api")]
                Subcommands::GenerateJWT => {
                    use crate::api::ApiConfigData;
                    let api_data = ApiConfigData::try_from(config.api.clone()).expect("invalid API config");
                    let claims = auth_helper::jwt::Claims::new(
                        ApiConfigData::ISSUER,
                        uuid::Uuid::new_v4().to_string(),
                        ApiConfigData::AUDIENCE,
                    )
                    .unwrap() // Panic: Cannot fail.
                    .expires_after_duration(api_data.jwt_expiration)
                    .map_err(crate::api::AuthError::InvalidJwt)?;
                    let exp_ts = time::OffsetDateTime::from_unix_timestamp(claims.exp.unwrap() as _).unwrap();
                    let jwt = auth_helper::jwt::JsonWebToken::new(claims, api_data.jwt_secret_key.as_ref())
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

                            tracing::info!("Computing the following analytics: {:?}", selected_analytics);

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
            AnalyticsChoice::LedgerOutputs => Box::new(LedgerOutputAnalytics),
            AnalyticsChoice::LedgerSize => Box::new(LedgerSizeAnalytics),
            AnalyticsChoice::OutputActivity => Box::new(OutputActivityAnalytics),
            AnalyticsChoice::ProtocolParameters => Box::new(ProtocolParametersAnalytics),
            AnalyticsChoice::UnclaimedTokens => Box::new(UnclaimedTokenAnalytics),
            AnalyticsChoice::UnlockConditions => Box::new(UnlockConditionAnalytics),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Subcommands {
    /// Generate a config from provided CLI parameters.
    CreateConfig {
        #[arg(short, long, value_name = "PATH", default_value = "./config.toml")]
        file_path: Option<String>,
    },
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
