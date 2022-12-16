// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod config;
mod measurement;

use std::ops::Deref;

use influxdb::{Client, ReadQuery};
use serde::de::DeserializeOwned;

pub use self::{config::InfluxDbConfig, measurement::InfluxDbMeasurement};

/// A wrapper for an InfluxDb [`Client`].
#[derive(Clone, Debug)]
pub struct InfluxClient(Client);

impl InfluxClient {
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

impl Deref for InfluxClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A wrapper for the influxdb [`Client`].
#[derive(Clone, Debug)]
pub struct InfluxDb {
    #[cfg(feature = "analytics")]
    analytics_client: InfluxClient,
    #[cfg(feature = "metrics")]
    metrics_client: InfluxClient,
    config: InfluxDbConfig,
}

impl InfluxDb {
    /// Create a new influx connection from config.
    pub async fn connect(config: &InfluxDbConfig) -> Result<Self, influxdb::Error> {
        #[cfg(feature = "analytics")]
        let analytics_client = {
            let client = InfluxClient(
                Client::new(&config.conn_url, &config.analytics_database_name).with_auth(&config.username, &config.password),
            );
            client.ping().await?;
            client
        };
        #[cfg(feature = "metrics")]
        let metrics_client = {
            let client = InfluxClient(
                Client::new(&config.conn_url, &config.metrics_database_name).with_auth(&config.username, &config.password),
            );
            client.ping().await?;
            client
        };
        Ok(Self {
            #[cfg(feature = "metrics")]
            metrics_client,
            #[cfg(feature = "analytics")]
            analytics_client,
            config: config.clone(),
        })
    }

    /// Get the analytics client.
    #[cfg(feature = "analytics")]
    pub fn analytics(&self) -> &InfluxClient {
        &self.analytics_client
    }

    /// Get the metrics client.
    #[cfg(feature = "metrics")]
    pub fn metrics(&self) -> &InfluxClient {
        &self.metrics_client
    }

    /// Get the config used to create the connection.
    pub fn config(&self) -> &InfluxDbConfig {
        &self.config
    }
}
