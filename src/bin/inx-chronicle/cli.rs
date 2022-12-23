// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::mongodb::config as mongodb;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::ChronicleConfig;

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
// #[command(author, version, about, next_display_order = None)]
#[command(author, version, about)]
pub struct ClArgs {
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
    pub mongodb_conn_str: String,
    /// The MongoDb username.
    #[arg(long, value_name = "USERNAME", env = "MONGODB_USERNAME", default_value = mongodb::DEFAULT_USERNAME)]
    pub mongodb_username: String,
    /// The MongoDb password.
    #[arg(long, value_name = "PASSWORD", env = "MONGODB_PASSWORD", default_value = mongodb::DEFAULT_PASSWORD)]
    pub mongodb_password: String,
    /// The MongoDb database name.
    #[arg(long, value_name = "NAME", default_value = mongodb::DEFAULT_DATABASE_NAME)]
    pub mongodb_database_name: String,
    /// The MongoDb minimum pool size.
    #[arg(long, value_name = "SIZE", default_value_t = mongodb::DEFAULT_MIN_POOL_SIZE)]
    pub mongodb_min_pool_size: u32,
}

impl From<&MongoDbArgs> for chronicle::db::MongoDbConfig {
    fn from(value: &MongoDbArgs) -> Self {
        Self {
            conn_str: value.mongodb_conn_str.clone(),
            username: value.mongodb_username.clone(),
            password: value.mongodb_password.clone(),
            database_name: value.mongodb_database_name.clone(),
            min_pool_size: value.mongodb_min_pool_size,
        }
    }
}

#[cfg(any(feature = "analytics", feature = "metrics"))]
use chronicle::db::influxdb::config as influxdb;

#[cfg(any(feature = "analytics", feature = "metrics"))]
#[derive(Args, Debug)]
pub struct InfluxDbArgs {
    /// The url pointing to an InfluxDb instance.
    #[arg(long, value_name = "URL", default_value = influxdb::DEFAULT_URL)]
    pub influxdb_url: String,
    /// The InfluxDb username.
    #[arg(long, value_name = "USERNAME", env = "INFLUXDB_USERNAME", default_value = influxdb::DEFAULT_USERNAME)]
    pub influxdb_username: String,
    /// The InfluxDb password.
    #[arg(long, value_name = "PASSWORD", env = "INFLUXDB_PASSWORD", default_value = influxdb::DEFAULT_PASSWORD)]
    pub influxdb_password: String,
    /// The Analytics database name.
    #[cfg(feature = "analytics")]
    #[arg(long, value_name = "NAME", default_value = influxdb::DEFAULT_ANALYTICS_DATABASE_NAME)]
    pub analytics_database_name: String,
    /// The Metrics database name.
    #[cfg(feature = "metrics")]
    #[arg(long, value_name = "NAME", default_value = influxdb::DEFAULT_METRICS_DATABASE_NAME)]
    pub metrics_database_name: String,
    /// Disable InfluxDb time-series analytics writes.
    #[cfg(feature = "analytics")]
    #[arg(long, default_value_t = !influxdb::DEFAULT_ANALYTICS_ENABLED)]
    pub disable_analytics: bool,
    /// Disable InfluxDb time-series metrics writes.
    #[cfg(feature = "metrics")]
    #[arg(long, default_value_t = !influxdb::DEFAULT_METRICS_ENABLED)]
    pub disable_metrics: bool,
}

#[cfg(any(feature = "analytics", feature = "metrics"))]
impl From<&InfluxDbArgs> for chronicle::db::influxdb::InfluxDbConfig {
    fn from(value: &InfluxDbArgs) -> Self {
        Self {
            url: value.influxdb_url.clone(),
            username: value.influxdb_username.clone(),
            password: value.influxdb_password.clone(),
            #[cfg(feature = "analytics")]
            analytics_enabled: !value.disable_analytics,
            #[cfg(feature = "analytics")]
            analytics_database_name: value.analytics_database_name.clone(),
            #[cfg(feature = "metrics")]
            metrics_enabled: !value.disable_metrics,
            #[cfg(feature = "metrics")]
            metrics_database_name: value.metrics_database_name.clone(),
        }
    }
}

#[cfg(all(feature = "stardust", feature = "inx"))]
use crate::stardust_inx::config as inx;

#[cfg(all(feature = "stardust", feature = "inx"))]
#[derive(Args, Debug)]
pub struct InxArgs {
    /// The address of the node INX interface Chronicle tries to connect to - if enabled.
    #[arg(long, value_name = "URL", default_value = inx::DEFAULT_URL)]
    pub inx_url: String,
    /// Time to wait until a new connection attempt is made.
    #[arg(long, value_name = "DURATION", value_parser = parse_duration, default_value = inx::DEFAULT_RETRY_INTERVAL)]
    pub inx_retry_interval: std::time::Duration,
    /// Maximum number of tries to establish an INX connection.
    #[arg(long, value_name = "COUNT", default_value_t = inx::DEFAULT_RETRY_COUNT)]
    pub inx_retry_count: usize,
    /// Milestone at which synchronization should begin. If set to `1` Chronicle will try to sync back until the
    /// genesis block. If set to `0` Chronicle will start syncing from the most recent milestone it received.
    #[arg(long, value_name = "START", default_value_t = inx::DEFAULT_SYNC_START)]
    pub inx_sync_start: u32,
    /// Disable the INX synchronization workflow.
    #[arg(long, default_value_t = !inx::DEFAULT_ENABLED)]
    pub disable_inx: bool,
}

#[cfg(all(feature = "stardust", feature = "inx"))]
impl From<&InxArgs> for inx::InxConfig {
    fn from(value: &InxArgs) -> Self {
        Self {
            enabled: !value.disable_inx,
            url: value.inx_url.clone(),
            conn_retry_interval: value.inx_retry_interval,
            conn_retry_count: value.inx_retry_count,
            sync_start_milestone: value.inx_sync_start.into(),
        }
    }
}

#[cfg(feature = "api")]
use crate::api::config as api;

#[cfg(feature = "api")]
#[derive(Args, Debug)]
pub struct ApiArgs {
    /// API listening port.
    #[arg(long, value_name = "PORT", default_value_t = api::DEFAULT_PORT)]
    pub api_port: u16,
    /// CORS setting.
    #[arg(long = "allow-origin", value_name = "IP", default_value = api::DEFAULT_ALLOW_ORIGINS)]
    pub allow_origins: Vec<String>,
    /// Public API routes.
    #[arg(long = "public-route", value_name = "ROUTE", default_value = api::DEFAULT_PUBLIC_ROUTES)]
    pub public_routes: Vec<String>,
    /// Maximum number of results returned by a single API call.
    #[arg(long, value_name = "SIZE", default_value_t = api::DEFAULT_MAX_PAGE_SIZE)]
    pub max_page_size: usize,
    /// JWT arguments.
    #[command(flatten)]
    pub jwt: JwtArgs,
    /// Disable REST API.
    #[arg(long, default_value_t = !api::DEFAULT_ENABLED)]
    pub disable_api: bool,
}

#[cfg(feature = "api")]
impl From<&ApiArgs> for api::ApiConfig {
    fn from(value: &ApiArgs) -> Self {
        Self {
            enabled: !value.disable_api,
            port: value.api_port,
            allow_origins: (&value.allow_origins).into(),
            jwt_password: value.jwt.jwt_password.clone(),
            jwt_salt: value.jwt.jwt_salt.clone(),
            jwt_identity_file: value.jwt.jwt_identity.clone(),
            jwt_expiration: value.jwt.jwt_expiration,
            max_page_size: value.max_page_size,
            public_routes: value.public_routes.clone(),
        }
    }
}

#[cfg(feature = "api")]
#[derive(Args, Debug)]
pub struct JwtArgs {
    /// The location of the identity file for JWT auth.
    #[arg(long, value_name = "FILEPATH", env = "JWT_IDENTITY", default_value = None)]
    pub jwt_identity: Option<String>,
    /// The password used for JWT authentication.
    #[arg(long, value_name = "PASSWORD", env = "JWT_PASSWORD", default_value = api::DEFAULT_JWT_PASSWORD)]
    pub jwt_password: String,
    /// The salt used for JWT authentication.
    #[arg(long, value_name = "SALT", env = "JWT_SALT", default_value = api::DEFAULT_JWT_SALT)]
    pub jwt_salt: String,
    /// The setting for when the (JWT) token expires.
    #[arg(long, value_name = "DURATION", value_parser = parse_duration, default_value = api::DEFAULT_JWT_EXPIRATION)]
    pub jwt_expiration: std::time::Duration,
}

#[cfg(feature = "loki")]
#[derive(Args, Debug)]
pub struct LokiArgs {
    /// The url pointing to a Grafana Loki instance.
    #[arg(long, value_name = "URL", default_value = crate::config::loki::DEFAULT_LOKI_URL)]
    pub loki_url: String,
    /// Disable Grafana Loki log writes.
    #[arg(long, default_value_t = !crate::config::loki::DEFAULT_LOKI_ENABLED)]
    pub disable_loki: bool,
}

#[cfg(feature = "loki")]
impl From<&LokiArgs> for crate::config::loki::LokiConfig {
    fn from(value: &LokiArgs) -> Self {
        Self {
            enabled: !value.disable_loki,
            url: value.loki_url.clone(),
        }
    }
}

#[cfg(any(all(feature = "stardust", feature = "inx"), feature = "api"))]
fn parse_duration(arg: &str) -> Result<std::time::Duration, humantime::DurationError> {
    arg.parse::<humantime::Duration>().map(Into::into)
}

impl ClArgs {
    /// Creates a [`ChronicleConfig`] from the given command-line arguments, environment variables, and defaults.
    pub fn get_config(&self) -> ChronicleConfig {
        ChronicleConfig {
            mongodb: (&self.mongodb).into(),
            #[cfg(any(feature = "analytics", feature = "metrics"))]
            influxdb: (&self.influxdb).into(),
            #[cfg(all(feature = "stardust", feature = "inx"))]
            inx: (&self.inx).into(),
            #[cfg(feature = "api")]
            api: (&self.api).into(),
            #[cfg(feature = "loki")]
            loki: (&self.loki).into(),
        }
    }

    /// Process subcommands and return whether the app should early exit.
    #[allow(unused)]
    #[allow(clippy::collapsible_match)]
    pub async fn process_subcommands(&self, config: &ChronicleConfig) -> eyre::Result<PostCommand> {
        if let Some(subcommand) = &self.subcommand {
            match subcommand {
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
                #[cfg(all(feature = "analytics", feature = "stardust", feature = "inx"))]
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
