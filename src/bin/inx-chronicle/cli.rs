// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

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
                    let mut analytics = analytics.iter().copied().collect::<HashSet<_>>();
                    let num_tasks = num_tasks.unwrap_or(1);
                    let mut join_set = tokio::task::JoinSet::<eyre::Result<()>>::new();
                    for i in 0..num_tasks {
                        let db = db.clone();
                        let influx_db = influx_db.clone();
                        let analytics = analytics.clone();
                        join_set.spawn(async move {
                            for index in (*start_milestone..*end_milestone).skip(i).step_by(num_tasks) {
                                let index = index.into();
                                if let Some(timestamp) = db
                                    .collection::<chronicle::db::collections::MilestoneCollection>()
                                    .get_milestone_timestamp(index)
                                    .await?
                                {
                                    #[cfg(feature = "metrics")]
                                    let start_time = std::time::Instant::now();

                                    futures::future::join_all(
                                        analytics
                                            .iter()
                                            .map(|analytic| analytic.gather(&db, &influx_db, index, timestamp)),
                                    )
                                    .await;

                                    #[cfg(feature = "metrics")]
                                    {
                                        let elapsed = start_time.elapsed();
                                        influx_db
                                            .insert(chronicle::db::collections::metrics::AnalyticsMetrics {
                                                time: chrono::Utc::now(),
                                                milestone_index: index,
                                                analytics_time: elapsed.as_millis() as u64,
                                                chronicle_version: std::env!("CARGO_PKG_VERSION").to_string(),
                                            })
                                            .await?;
                                    }
                                    tracing::info!("Finished analytics for milestone {}", index);
                                } else {
                                    tracing::info!("No milestone in database for index {}", index);
                                }
                            }
                            Ok(())
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
        /// The list of analytics to fill.
        #[arg(short, long, value_enum, default_values_t = Analytic::all())]
        analytics: Vec<Analytic>,
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

#[cfg(all(feature = "analytics", feature = "stardust"))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum Analytic {
    AddressActivity,
    Addresses,
    BaseTokens,
    LedgerOutputs,
    Aliases,
    Nfts,
    LedgerSize,
    UnclaimedTokens,
    UnlockConditions,
    PayloadActivity,
    TransactionActivity,
    ProtocolParams,
}

#[cfg(all(feature = "analytics", feature = "stardust"))]
impl Analytic {
    fn all() -> Vec<Self> {
        vec![
            Self::AddressActivity,
            Self::Addresses,
            Self::BaseTokens,
            Self::LedgerOutputs,
            Self::Aliases,
            Self::Nfts,
            Self::LedgerSize,
            Self::UnclaimedTokens,
            Self::UnlockConditions,
            Self::PayloadActivity,
            Self::TransactionActivity,
            Self::ProtocolParams,
        ]
    }

    async fn gather(
        &self,
        db: &chronicle::db::MongoDb,
        influx_db: &chronicle::db::influxdb::InfluxDb,
        index: chronicle::types::tangle::MilestoneIndex,
        timestamp: chronicle::types::stardust::milestone::MilestoneTimestamp,
    ) -> eyre::Result<()> {
        match self {
            Analytic::AddressActivity => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_address_activity_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::Addresses => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_address_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::BaseTokens => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_base_token_activity_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::LedgerOutputs => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_ledger_output_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::Aliases => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_alias_output_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::Nfts => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_nft_output_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::LedgerSize => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_ledger_size_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::UnclaimedTokens => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_unclaimed_token_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::UnlockConditions => {
                let analytics = db
                    .collection::<chronicle::db::collections::OutputCollection>()
                    .get_unlock_condition_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::PayloadActivity => {
                let analytics = db
                    .collection::<chronicle::db::collections::BlockCollection>()
                    .get_payload_activity_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::TransactionActivity => {
                let analytics = db
                    .collection::<chronicle::db::collections::BlockCollection>()
                    .get_transaction_activity_analytics(index)
                    .await?;
                influx_db.insert_analytics(timestamp, index, analytics).await?;
            }
            Analytic::ProtocolParams => {
                let analytics = db
                    .collection::<chronicle::db::collections::ProtocolUpdateCollection>()
                    .get_protocol_parameters_for_milestone_index(index)
                    .await?
                    .map(|p| p.parameters);
                if let Some(analytics) = analytics {
                    influx_db.insert_analytics(timestamp, index, analytics).await?;
                }
            }
        }
        Ok(())
    }
}
