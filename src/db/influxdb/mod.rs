// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod measurement;

use std::ops::Deref;

use influxdb::{Client, ReadQuery};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use self::measurement::InfluxDbMeasurement;

/// A wrapper for the influxdb [`Client`].
#[derive(Clone, Debug)]
pub struct InfluxDb(Client);

impl InfluxDb {
    /// Create a new influx connection from config.
    pub async fn connect(config: &InfluxDbConfig) -> Result<Self, influxdb::Error> {
        let client = Client::new(&config.url, &config.database_name).with_auth(&config.username, &config.password);
        client.ping().await?;
        Ok(Self(client))
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
}

impl Deref for InfluxDb {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The influxdb [`Client`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InfluxDbConfig {
    /// The address of the InfluxDb instance.
    pub url: String,
    /// The InfluxDb username.
    pub username: String,
    /// The InfluxDb password.
    pub password: String,
    /// The name of the database to connect to.
    pub database_name: String,
    /// Whether to enable influx writes.
    pub enabled: bool,
}

impl Default for InfluxDbConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8086".to_string(),
            database_name: "chronicle_analytics".to_string(),
            username: "root".to_string(),
            password: "password".to_string(),
            enabled: true,
        }
    }
}
