// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `MongoDb` config and its defaults.

use mongodb::{
    error::Error,
    options::{ConnectionString, HostInfo},
};
use serde::{Deserialize, Serialize};

/// The default connection string of the database.
pub const DEFAULT_CONN_STR: &str = "mongodb://localhost:27017";
/// The default MongoDB username.
pub const DEFAULT_USERNAME: &str = "root";
/// The default MongoDB password.
pub const DEFAULT_PASSWORD: &str = "root";
/// The default name of the database to connect to.
pub const DEFAULT_DATABASE_NAME: &str = "chronicle";
/// The default minimum amount of connections in the pool.
pub const DEFAULT_MIN_POOL_SIZE: u32 = 2;

/// The [`MongoDb`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MongoDbConfig {
    /// The connection string of the database.
    pub conn_str: String,
    /// The MongoDB username.
    pub username: String,
    /// The MongoDB password.
    pub password: String,
    /// The name of the database to connect to.
    pub database_name: String,
    /// The minimum amount of connections in the pool.
    pub min_pool_size: u32,
}

impl MongoDbConfig {
    /// Get the hosts portion of the connection string.
    pub fn hosts_str(&self) -> Result<String, Error> {
        let hosts = ConnectionString::parse(&self.conn_str)?.host_info;
        Ok(match hosts {
            HostInfo::HostIdentifiers(hosts) => hosts.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
            HostInfo::DnsRecord(hostname) => hostname,
            _ => unreachable!(),
        })
    }
}

impl Default for MongoDbConfig {
    fn default() -> Self {
        Self {
            conn_str: DEFAULT_CONN_STR.to_string(),
            username: DEFAULT_USERNAME.to_string(),
            password: DEFAULT_PASSWORD.to_string(),
            database_name: DEFAULT_DATABASE_NAME.to_string(),
            min_pool_size: DEFAULT_MIN_POOL_SIZE,
        }
    }
}
