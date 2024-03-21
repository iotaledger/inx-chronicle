// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `MongoDb` config and its defaults.

use mongodb::{
    error::Error,
    options::{ConnectionString, HostInfo},
};

/// The default connection string of the database.
pub const DEFAULT_CONN_STR: &str = "mongodb://localhost:27017";
/// The default name of the database to connect to.
pub const DEFAULT_DATABASE_NAME: &str = "chronicle";

/// The [`super::MongoDb`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MongoDbConfig {
    /// The connection string of the database.
    pub conn_str: String,
    /// The name of the database to connect to.
    pub database_name: String,
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
            database_name: DEFAULT_DATABASE_NAME.to_string(),
        }
    }
}
