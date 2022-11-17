// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod measurement;

use std::ops::Deref;

use influxdb::{Client, ReadQuery};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use self::measurement::InfluxDbMeasurement;

/// A wrapper for the influxdb [`Client`].
#[derive(Clone, Debug)]
pub struct InfluxDb {
    client: Client,
    config: InfluxDbConfig,
}

impl InfluxDb {
    /// Create a new influx connection from config.
    pub async fn connect(config: &InfluxDbConfig) -> Result<Self, influxdb::Error> {
        let client = Client::new(&config.url, &config.database_name).with_auth(&config.username, &config.password);
        client.ping().await?;
        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    /// Insert a measurement value.
    pub async fn insert<M: InfluxDbMeasurement>(&self, value: M) -> Result<(), influxdb::Error> {
        self.query(value.into_query(M::NAME)).await?;
        Ok(())
    }

    /// Select measurements using the provided query.
    pub async fn select<T: 'static + DeserializeOwned + Send + Sync>(
        &self,
        query: ReadQuery,
    ) -> Result<Box<dyn Iterator<Item = T>>, influxdb::Error> {
        Ok(Box::new(
            self.json_query(query)
                .await?
                .deserialize_next::<T>()?
                .series
                .into_iter()
                .map(|mut res| res.values.remove(0)),
        ))
    }

    /// Get the config used to create the connection.
    pub fn config(&self) -> &InfluxDbConfig {
        &self.config
    }
}

impl Deref for InfluxDb {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

/// The influxdb [`Client`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InfluxDbConfig {
    /// The address of the InfluxDb instance.
    pub url: String,
    /// The InfluxDb username.
    pub username: String,
    /// The InfluxDb password.
    pub password: String,
    /// The name of the database to connect to.
    pub database_name: String,
    /// Whether to enable influx metrics writes.
    pub metrics_enabled: bool,
    /// Whether to enable influx analytics writes.
    pub analytics_enabled: bool,
}

impl InfluxDbConfig {
    /// Applies the corresponding user config.
    #[allow(clippy::option_map_unit_fn)]
    pub fn apply_user_config(&mut self, user_config: InfluxDbUserConfig) {
        user_config.url.map(|v| self.url = v);
        user_config.username.map(|v| self.username = v);
        user_config.password.map(|v| self.password = v);
        user_config.database_name.map(|v| self.database_name = v);
        user_config.metrics_enabled.map(|v| self.metrics_enabled = v);
        user_config.analytics_enabled.map(|v| self.analytics_enabled = v);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(missing_docs)]
pub struct InfluxDbUserConfig {
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database_name: Option<String>,
    pub metrics_enabled: Option<bool>,
    pub analytics_enabled: Option<bool>,
}
