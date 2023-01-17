// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `InfluxDb` config and its defaults.

/// The default InfluxDb URL to connect to.
pub const DEFAULT_URL: &str = "http://localhost:8086";
/// The default InfluxDb username.
pub const DEFAULT_USERNAME: &str = "root";
/// The default InfluxDb password.
pub const DEFAULT_PASSWORD: &str = "password";
/// The default whether to enable influx analytics writes.
#[cfg(feature = "analytics")]
pub const DEFAULT_ANALYTICS_ENABLED: bool = true;
/// The default name of the analytics database to connect to.
#[cfg(feature = "analytics")]
pub const DEFAULT_ANALYTICS_DATABASE_NAME: &str = "chronicle_analytics";
/// The default whether to enable influx metrics writes.
#[cfg(feature = "metrics")]
pub const DEFAULT_METRICS_ENABLED: bool = true;
/// The default name of the metrics database to connect to.
#[cfg(feature = "metrics")]
pub const DEFAULT_METRICS_DATABASE_NAME: &str = "chronicle_metrics";

/// The influxdb [`influxdb::Client`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InfluxDbConfig {
    /// The address of the InfluxDb instance.
    pub url: String,
    /// The InfluxDb username.
    pub username: String,
    /// The InfluxDb password.
    pub password: String,
    /// Whether to enable influx analytics writes.
    #[cfg(feature = "analytics")]
    pub analytics_enabled: bool,
    /// The name of the database to insert analytics.
    #[cfg(feature = "analytics")]
    pub analytics_database_name: String,
    /// The selected analytics to compute.
    #[cfg(feature = "analytics")]
    pub analytics: Vec<AnalyticsChoice>,
    /// Whether to enable influx metrics writes.
    #[cfg(feature = "metrics")]
    pub metrics_enabled: bool,
    /// The name of the database to insert metrics.
    #[cfg(feature = "metrics")]
    pub metrics_database_name: String,
}

impl Default for InfluxDbConfig {
    fn default() -> Self {
        Self {
            url: DEFAULT_URL.to_string(),
            username: DEFAULT_USERNAME.to_string(),
            password: DEFAULT_PASSWORD.to_string(),
            #[cfg(feature = "analytics")]
            analytics_enabled: DEFAULT_ANALYTICS_ENABLED,
            #[cfg(feature = "analytics")]
            analytics_database_name: DEFAULT_ANALYTICS_DATABASE_NAME.to_string(),
            #[cfg(feature = "analytics")]
            analytics: Vec::new(),
            #[cfg(feature = "metrics")]
            metrics_enabled: DEFAULT_METRICS_ENABLED,
            #[cfg(feature = "metrics")]
            metrics_database_name: DEFAULT_METRICS_DATABASE_NAME.to_string(),
        }
    }
}

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, clap::ValueEnum)]
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

#[cfg(feature = "analytics")]
impl From<AnalyticsChoice> for Box<dyn crate::db::collections::analytics::Analytic> {
    fn from(value: AnalyticsChoice) -> Self {
        use crate::db::collections::analytics::{
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
