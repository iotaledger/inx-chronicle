// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod measurement;

use std::ops::{Add, AddAssign, Deref, DerefMut, Mul, Sub, SubAssign};

use decimal::d128;
use influxdb::{Client, ReadQuery, Type};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use self::measurement::InfluxDbMeasurement;

/// A wrapper for the influxdb [`Client`].
#[derive(Clone, Debug)]
pub struct InfluxDb(Client);

impl InfluxDb {
    /// Create a new influx connection from config.
    pub async fn connect(config: &InfluxDbConfig) -> Result<Self, influxdb::Error> {
        let client = Client::new(&config.url, &config.database_name).with_auth(
            config.username.as_deref().unwrap_or_default(),
            config.password.as_deref().unwrap_or_default(),
        );
        client.ping().await?;
        // client
        //     .query(ReadQuery::new(format!("CREATE DATABASE IF NOT EXISTS {}", config.database_name)))
        //     .await?;
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
    /// The bind address of the database.
    pub url: String,
    /// The InfluxDb username.
    pub username: Option<String>,
    /// The InfluxDb password.
    pub password: Option<String>,
    /// The name of the database to connect to.
    pub database_name: String,
}

impl Default for InfluxDbConfig {
    fn default() -> Self {
        Self {
            url: "localhost:8086".to_string(),
            database_name: "chronicle_analytics".to_string(),
            username: None,
            password: None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Decimal128(pub d128);

impl AsRef<d128> for Decimal128 {
    fn as_ref(&self) -> &d128 {
        &self.0
    }
}

impl Deref for Decimal128 {
    type Target = d128;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Decimal128 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Decimal128> for Type {
    fn from(value: Decimal128) -> Self {
        Type::Text(value.0.to_string())
    }
}

impl<T: AsRef<d128>> Add<T> for Decimal128 {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs.as_ref())
    }
}

impl<T: AsRef<d128>> AddAssign<T> for Decimal128 {
    fn add_assign(&mut self, rhs: T) {
        *self = *self + rhs
    }
}

impl<T: AsRef<d128>> Sub<T> for Decimal128 {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Self(self.0 - rhs.as_ref())
    }
}

impl<T: AsRef<d128>> SubAssign<T> for Decimal128 {
    fn sub_assign(&mut self, rhs: T) {
        *self = *self - rhs
    }
}

impl<T: AsRef<d128>> Mul<T> for Decimal128 {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0 * rhs.as_ref())
    }
}
