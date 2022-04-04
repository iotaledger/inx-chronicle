// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{path::PathBuf, time::Duration};

use cfg_if::cfg_if;
use mongodb::{
    bson::Document,
    options::{ClientOptions, ReadConcern, ReadPreferenceOptions, ServerApi, WriteConcern},
};
use serde::{Deserialize, Serialize};

/// A clone of the [`ClientOptions`] structure from the [`mongodb`] crate
/// which can be Serialized.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MongoConfig {
    /// The initial list of seeds that the Client should connect to.
    ///
    /// Note that by default, the driver will autodiscover other nodes in the cluster. To connect
    /// directly to a single server (rather than autodiscovering the rest of the cluster), set the
    /// `direct_connection` field to `true`.
    #[serde(default = "default_hosts")]
    pub hosts: Vec<ServerAddress>,

    /// The application name that the Client will send to the server as part of the handshake. This
    /// can be used in combination with the server logs to determine which Client is connected to a
    /// server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,

    /// The compressors that the Client is willing to use in the order they are specified
    /// in the configuration.  The Client sends this list of compressors to the server.
    /// The server responds with the intersection of its supported list of compressors.
    /// The order of compressors indicates preference of compressors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressors: Option<Vec<Compressor>>,

    /// The connect timeout passed to each underlying TcpStream when attempting to connect to the
    /// server.
    ///
    /// The default value is 10 seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_timeout: Option<Duration>,

    /// The credential to use for authenticating connections made by this client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<Credential>,

    /// Specifies whether the Client should directly connect to a single host rather than
    /// autodiscover all servers in the cluster.
    ///
    /// The default value is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_connection: Option<bool>,

    /// Extra information to append to the driver version in the metadata of the handshake with the
    /// server. This should be used by libraries wrapping the driver, e.g. ODMs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_info: Option<DriverInfo>,

    /// The amount of time each monitoring thread should wait between sending an isMaster command
    /// to its respective server.
    ///
    /// The default value is 10 seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heartbeat_freq: Option<Duration>,

    /// When running a read operation with a ReadPreference that allows selecting secondaries,
    /// `local_threshold` is used to determine how much longer the average round trip time between
    /// the driver and server is allowed compared to the least round trip time of all the suitable
    /// servers. For example, if the average round trip times of the suitable servers are 5 ms, 10
    /// ms, and 15 ms, and the local threshold is 8 ms, then the first two servers are within the
    /// latency window and could be chosen for the operation, but the last one is not.
    ///
    /// A value of zero indicates that there is no latency window, so only the server with the
    /// lowest average round trip time is eligible.
    ///
    /// The default value is 15 ms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_threshold: Option<Duration>,

    /// The amount of time that a connection can remain idle in a connection pool before being
    /// closed. A value of zero indicates that connections should not be closed due to being idle.
    ///
    /// By default, connections will not be closed due to being idle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_idle_time: Option<Duration>,

    /// The maximum amount of connections that the Client should allow to be created in a
    /// connection pool for a given server. If an operation is attempted on a server while
    /// `max_pool_size` connections are checked out, the operation will block until an in-progress
    /// operation finishes and its connection is checked back into the pool.
    ///
    /// The default value is 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_pool_size: Option<u32>,

    /// The minimum number of connections that should be available in a server's connection pool at
    /// a given time. If fewer than `min_pool_size` connections are in the pool, connections will
    /// be added to the pool in the background until `min_pool_size` is reached.
    ///
    /// The default value is 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_pool_size: Option<u32>,

    /// Specifies the default read concern for operations performed on the Client. See the
    /// ReadConcern type documentation for more details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_concern: Option<ReadConcern>,

    /// The name of the replica set that the Client should connect to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repl_set_name: Option<String>,

    /// Whether or not the client should retry a read operation if the operation fails.
    ///
    /// The default value is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_reads: Option<bool>,

    /// Whether or not the client should retry a write operation if the operation fails.
    ///
    /// The default value is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_writes: Option<bool>,

    /// The default selection criteria for operations performed on the Client. See the
    /// SelectionCriteria type documentation for more details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_criteria: Option<ReadPreference>,

    /// The declared API version for this client.
    /// The declared API version is applied to all commands run through the client, including those
    /// sent through any handle derived from the client.
    ///
    /// Specifying versioned API options in the command document passed to `run_command` AND
    /// declaring an API version on the client is not supported and is considered undefined
    /// behaviour. To run any command with a different API version or without declaring one, create
    /// a separate client that declares the appropriate API version.
    ///
    /// For more information, see the [Versioned API](
    /// https://docs.mongodb.com/v5.0/reference/versioned-api/) manual page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_api: Option<ServerApi>,

    /// The amount of time the Client should attempt to select a server for an operation before
    /// timing outs
    ///
    /// The default value is 30 seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_selection_timeout: Option<Duration>,

    /// Default database for this client.
    ///
    /// By default, no default database is specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_database: Option<String>,

    /// The TLS configuration for the Client to use in its connections with the server.
    ///
    /// By default, TLS is disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<Tls>,

    /// Specifies the default write concern for operations performed on the Client. See the
    /// WriteConcern type documentation for more details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_concern: Option<WriteConcern>,
}

#[allow(missing_docs)]
impl MongoConfig {
    pub fn with_hosts(mut self, hosts: Vec<impl Into<ServerAddress>>) -> Self {
        self.hosts = hosts.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name.replace(app_name.into());
        self
    }

    pub fn with_compressors(mut self, compressors: Vec<impl Into<Compressor>>) -> Self {
        self.compressors
            .replace(compressors.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout.replace(connect_timeout);
        self
    }

    pub fn with_credential(mut self, credential: impl Into<Credential>) -> Self {
        self.credential.replace(credential.into());
        self
    }

    pub fn with_direct_connection(mut self, direct_connection: bool) -> Self {
        self.direct_connection.replace(direct_connection);
        self
    }

    pub fn with_driver_info(mut self, driver_info: impl Into<DriverInfo>) -> Self {
        self.driver_info.replace(driver_info.into());
        self
    }

    pub fn with_heartbeat_frequency(mut self, heartbeat_frequency: Duration) -> Self {
        self.heartbeat_freq.replace(heartbeat_frequency);
        self
    }

    pub fn with_local_threshold(mut self, local_threshold: Duration) -> Self {
        self.local_threshold.replace(local_threshold);
        self
    }

    pub fn with_max_idle_time(mut self, max_idle_time: Duration) -> Self {
        self.max_idle_time.replace(max_idle_time);
        self
    }

    pub fn with_max_pool_size(mut self, max_pool_size: u32) -> Self {
        self.max_pool_size.replace(max_pool_size);
        self
    }

    pub fn with_min_pool_size(mut self, min_pool_size: u32) -> Self {
        self.min_pool_size.replace(min_pool_size);
        self
    }

    pub fn with_read_concern(mut self, read_concern: impl Into<ReadConcern>) -> Self {
        self.read_concern.replace(read_concern.into());
        self
    }

    pub fn with_replica_set_name(mut self, repl_set_name: impl Into<String>) -> Self {
        self.repl_set_name.replace(repl_set_name.into());
        self
    }

    pub fn with_retry_reads(mut self, retry_reads: bool) -> Self {
        self.retry_reads.replace(retry_reads);
        self
    }

    pub fn with_retry_writes(mut self, retry_writes: bool) -> Self {
        self.retry_writes.replace(retry_writes);
        self
    }

    pub fn with_selection_criteria(mut self, selection_criteria: impl Into<ReadPreference>) -> Self {
        self.selection_criteria.replace(selection_criteria.into());
        self
    }

    pub fn with_server_api(mut self, server_api: impl Into<ServerApi>) -> Self {
        self.server_api.replace(server_api.into());
        self
    }

    pub fn with_server_selection_timeout(mut self, server_selection_timeout: Duration) -> Self {
        self.server_selection_timeout.replace(server_selection_timeout);
        self
    }

    pub fn with_default_database(mut self, default_database: impl Into<String>) -> Self {
        self.default_database.replace(default_database.into());
        self
    }

    pub fn with_tls(mut self, tls: impl Into<Tls>) -> Self {
        self.tls.replace(tls.into());
        self
    }

    pub fn with_write_concern(mut self, write_concern: impl Into<WriteConcern>) -> Self {
        self.write_concern.replace(write_concern.into());
        self
    }

    pub fn build(self) -> ClientOptions {
        self.into()
    }
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            hosts: default_hosts(),
            app_name: Default::default(),
            compressors: Default::default(),
            connect_timeout: Default::default(),
            credential: Default::default(),
            direct_connection: Default::default(),
            driver_info: Default::default(),
            heartbeat_freq: Default::default(),
            local_threshold: Default::default(),
            max_idle_time: Default::default(),
            max_pool_size: Default::default(),
            min_pool_size: Default::default(),
            read_concern: Default::default(),
            repl_set_name: Default::default(),
            retry_reads: Default::default(),
            retry_writes: Default::default(),
            selection_criteria: Default::default(),
            server_api: Default::default(),
            server_selection_timeout: Default::default(),
            default_database: Default::default(),
            tls: Default::default(),
            write_concern: Default::default(),
        }
    }
}

impl From<MongoConfig> for ClientOptions {
    fn from(config: MongoConfig) -> Self {
        Self::builder()
            .hosts(config.hosts.into_iter().map(Into::into).collect::<Vec<_>>())
            .app_name(config.app_name)
            .compressors(config.compressors.map(|v| v.into_iter().map(Into::into).collect()))
            .connect_timeout(config.connect_timeout)
            .credential(config.credential.map(Into::into))
            .direct_connection(config.direct_connection)
            .driver_info(config.driver_info.map(Into::into))
            .heartbeat_freq(config.heartbeat_freq)
            .local_threshold(config.local_threshold)
            .max_idle_time(config.max_idle_time)
            .max_pool_size(config.max_pool_size)
            .min_pool_size(config.min_pool_size)
            .read_concern(config.read_concern)
            .repl_set_name(config.repl_set_name)
            .retry_reads(config.retry_reads)
            .retry_writes(config.retry_writes)
            .selection_criteria(config.selection_criteria.map(Into::into))
            .server_api(config.server_api)
            .server_selection_timeout(config.server_selection_timeout)
            .default_database(config.default_database)
            .tls(config.tls.map(Into::into))
            .write_concern(config.write_concern)
            .build()
    }
}

/// An enum representing the address of a MongoDB server.
///
/// Currently this just supports addresses that can be connected to over TCP, but alternative
/// address types may be supported in the future (e.g. Unix Domain Socket paths).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServerAddress {
    /// A TCP/IP host and port combination.
    Tcp {
        /// The hostname or IP address where the MongoDB server can be found.
        host: String,

        /// The TCP port that the MongoDB server is listening on.
        ///
        /// The default is 27017.
        port: Option<u16>,
    },
}

impl Default for ServerAddress {
    fn default() -> Self {
        Self::Tcp {
            host: "localhost".to_string(),
            port: Some(27017),
        }
    }
}

impl From<ServerAddress> for mongodb::options::ServerAddress {
    fn from(address: ServerAddress) -> Self {
        match address {
            ServerAddress::Tcp { host, port } => Self::Tcp { host, port },
        }
    }
}

impl From<mongodb::options::ServerAddress> for ServerAddress {
    fn from(addr: mongodb::options::ServerAddress) -> Self {
        match addr {
            mongodb::options::ServerAddress::Tcp { host, port } => ServerAddress::Tcp { host, port },
            _ => panic!("Unsupported ServerAddress variant"),
        }
    }
}

fn default_hosts() -> Vec<ServerAddress> {
    vec![ServerAddress::default()]
}

/// Enum representing supported compressor algorithms.
/// Used for compressing and decompressing messages sent to and read from the server.
/// For compressors that take a `level`, use `None` to indicate the default level.
/// Higher `level` indicates more compression (and slower).
/// Requires `zstd-compression` feature flag to use `Zstd` compressor,
/// `zlib-compression` feature flag to use `Zlib` compressor, and
/// `snappy-compression` feature flag to use `Snappy` Compressor.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Compressor {
    /// Zstd compressor.  Requires Rust version 1.54.
    /// See [`Zstd`](http://facebook.github.io/zstd/zstd_manual.html) for more information
    Zstd {
        /// Zstd compression level
        level: Option<i32>,
    },
    /// Zlib compressor.
    /// See [`Zlib`](https://zlib.net/) for more information.
    Zlib {
        /// Zlib compression level
        level: Option<i32>,
    },
    /// Snappy compressor.
    /// See [`Snappy`](http://google.github.io/snappy/) for more information.
    Snappy,
}

impl From<Compressor> for mongodb::options::Compressor {
    fn from(compressor: Compressor) -> Self {
        match compressor {
            Compressor::Zstd {
                #[cfg(feature = "mongodb/zstd-compression")]
                level,
                #[cfg(not(feature = "mongodb/zstd-compression"))]
                    level: _,
            } => {
                cfg_if! {
                    if #[cfg(feature = "mongodb/zstd-compression")] {
                        Self::Zstd { level }
                    } else {
                        panic!("mongodb/zstd-compression feature flag not enabled")
                    }
                }
            }
            Compressor::Zlib {
                #[cfg(feature = "mongodb/zlib-compression")]
                level,
                #[cfg(not(feature = "mongodb/zlib-compression"))]
                    level: _,
            } => {
                cfg_if! {
                    if #[cfg(feature = "mongodb/zlib-compression")] {
                        Self::Zlib { level }
                    } else {
                        panic!("mongodb/zlib-compression feature flag not enabled")
                    }
                }
            }
            Compressor::Snappy => {
                cfg_if! {
                    if #[cfg(feature = "mongodb/snappy-compression")] {
                        Self::Snappy
                    } else {
                        panic!("mongodb/snappy-compression feature flag not enabled")
                    }
                }
            }
        }
    }
}

impl From<mongodb::options::Compressor> for Compressor {
    fn from(compressor: mongodb::options::Compressor) -> Self {
        match compressor {
            #[cfg(feature = "mongodb/zstd-compression")]
            mongodb::options::Compressor::Zstd { level } => Compressor::Zstd { level },
            #[cfg(feature = "mongodb/zlib-compression")]
            mongodb::options::Compressor::Zlib { level } => Compressor::Zlib { level },
            #[cfg(feature = "mongodb/snappy-compression")]
            mongodb::options::Compressor::Snappy => Compressor::Snappy,
            _ => panic!("Unsupported Compressor variant"),
        }
    }
}

/// Specifies whether TLS configuration should be used with the operations that the
/// [`Client`](mongodb::Client) performs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Tls {
    /// Enable TLS with the specified options.
    Enabled(TlsOptions),

    /// Disable TLS.
    Disabled,
}

impl From<Tls> for mongodb::options::Tls {
    fn from(tls: Tls) -> Self {
        match tls {
            Tls::Enabled(tls_options) => Self::Enabled(tls_options.into()),
            Tls::Disabled => Self::Disabled,
        }
    }
}

impl From<mongodb::options::Tls> for Tls {
    fn from(tls: mongodb::options::Tls) -> Self {
        match tls {
            mongodb::options::Tls::Enabled(tls_options) => Tls::Enabled(tls_options.into()),
            mongodb::options::Tls::Disabled => Tls::Disabled,
        }
    }
}

/// Specifies the TLS configuration that the [`Client`](mongodb::Client) should use.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[allow(missing_docs)]
pub struct TlsOptions {
    /// Whether or not the [`Client`](mongodb::Client) should return an error if the server
    /// presents an invalid certificate. This setting should _not_ be set to `true` in
    /// production; it should only be used for testing.
    ///
    /// The default value is to error when the server presents an invalid certificate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_invalid_certificates: Option<bool>,

    /// The path to the CA file that the [`Client`](mongodb::Client) should use for TLS. If
    /// none is specified, then the driver will use the Mozilla root certificates from the
    /// `webpki-roots` crate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_file_path: Option<PathBuf>,

    /// The path to the certificate file that the [`Client`](mongodb::Client) should present
    /// to the server to verify its identify. If none is specified, then the
    /// [`Client`](mongodb::Client) will not attempt to verify its identity to the
    /// server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_key_file_path: Option<PathBuf>,
}

#[allow(missing_docs)]
impl TlsOptions {
    pub fn with_allow_invalid_certificates(mut self, allow_invalid_certificates: bool) -> Self {
        self.allow_invalid_certificates = Some(allow_invalid_certificates);
        self
    }

    pub fn with_ca_file_path(mut self, ca_file_path: PathBuf) -> Self {
        self.ca_file_path = Some(ca_file_path);
        self
    }

    pub fn with_cert_key_file_path(mut self, cert_key_file_path: PathBuf) -> Self {
        self.cert_key_file_path = Some(cert_key_file_path);
        self
    }
}

impl From<TlsOptions> for mongodb::options::TlsOptions {
    fn from(tls_options: TlsOptions) -> Self {
        Self::builder()
            .allow_invalid_certificates(tls_options.allow_invalid_certificates)
            .ca_file_path(tls_options.ca_file_path)
            .cert_key_file_path(tls_options.cert_key_file_path)
            .build()
    }
}

impl From<mongodb::options::TlsOptions> for TlsOptions {
    fn from(tls_options: mongodb::options::TlsOptions) -> Self {
        TlsOptions {
            allow_invalid_certificates: tls_options.allow_invalid_certificates,
            ca_file_path: tls_options.ca_file_path,
            cert_key_file_path: tls_options.cert_key_file_path,
        }
    }
}

/// Specifies how the driver should route a read operation to members of a replica set.
///
/// If applicable, `tag_sets` can be used to target specific nodes in a replica set, and
/// `max_staleness` specifies the maximum lag behind the primary that a secondary can be to remain
/// eligible for the operation. The max staleness value maps to the `maxStalenessSeconds` MongoDB
/// option and will be sent to the server as an integer number of seconds.
///
/// See the [MongoDB docs](https://docs.mongodb.com/manual/core/read-preference) for more details.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ReadPreference {
    /// Only route this operation to the primary.
    Primary,

    /// Only route this operation to a secondary.
    Secondary { options: ReadPreferenceOptions },

    /// Route this operation to the primary if it's available, but fall back to the secondaries if
    /// not.
    PrimaryPreferred { options: ReadPreferenceOptions },

    /// Route this operation to a secondary if one is available, but fall back to the primary if
    /// not.
    SecondaryPreferred { options: ReadPreferenceOptions },

    /// Route this operation to the node with the least network latency regardless of whether it's
    /// the primary or a secondary.
    Nearest { options: ReadPreferenceOptions },
}

impl From<ReadPreference> for mongodb::options::SelectionCriteria {
    fn from(read_preference: ReadPreference) -> Self {
        Self::ReadPreference(read_preference.into())
    }
}

impl From<mongodb::options::SelectionCriteria> for ReadPreference {
    fn from(selection_criteria: mongodb::options::SelectionCriteria) -> Self {
        match selection_criteria {
            mongodb::options::SelectionCriteria::ReadPreference(read_preference) => read_preference.into(),
            _ => panic!("Unsupported SelectionCriteria variant"),
        }
    }
}

impl From<ReadPreference> for mongodb::options::ReadPreference {
    fn from(read_preference: ReadPreference) -> Self {
        match read_preference {
            ReadPreference::Primary => Self::Primary,
            ReadPreference::Secondary { options } => Self::Secondary { options },
            ReadPreference::PrimaryPreferred { options } => Self::PrimaryPreferred { options },
            ReadPreference::SecondaryPreferred { options } => Self::SecondaryPreferred { options },
            ReadPreference::Nearest { options } => Self::Nearest { options },
        }
    }
}

impl From<mongodb::options::ReadPreference> for ReadPreference {
    fn from(read_preference: mongodb::options::ReadPreference) -> Self {
        match read_preference {
            mongodb::options::ReadPreference::Primary => ReadPreference::Primary,
            mongodb::options::ReadPreference::Secondary { options } => ReadPreference::Secondary { options },
            mongodb::options::ReadPreference::PrimaryPreferred { options } => {
                ReadPreference::PrimaryPreferred { options }
            }
            mongodb::options::ReadPreference::SecondaryPreferred { options } => {
                ReadPreference::SecondaryPreferred { options }
            }
            mongodb::options::ReadPreference::Nearest { options } => ReadPreference::Nearest { options },
        }
    }
}

/// Extra information to append to the driver version in the metadata of the handshake with the
/// server. This should be used by libraries wrapping the driver, e.g. ODMs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DriverInfo {
    /// The name of the library wrapping the driver.
    pub name: String,

    /// The version of the library wrapping the driver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Optional platform information for the wrapping driver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

#[allow(missing_docs)]
impl DriverInfo {
    pub fn named(name: String) -> Self {
        Self {
            name,
            version: None,
            platform: None,
        }
    }

    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    pub fn with_platform(mut self, platform: String) -> Self {
        self.platform = Some(platform);
        self
    }
}

impl From<DriverInfo> for mongodb::options::DriverInfo {
    fn from(driver_info: DriverInfo) -> Self {
        Self::builder()
            .name(driver_info.name)
            .version(driver_info.version)
            .platform(driver_info.platform)
            .build()
    }
}

impl From<mongodb::options::DriverInfo> for DriverInfo {
    fn from(driver_info: mongodb::options::DriverInfo) -> Self {
        DriverInfo {
            name: driver_info.name,
            version: driver_info.version,
            platform: driver_info.platform,
        }
    }
}

/// A struct containing authentication information.
///
/// Some fields (mechanism and source) may be omitted and will either be negotiated or assigned a
/// default value, depending on the values of other fields in the credential.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Credential {
    /// The username to authenticate with. This applies to all mechanisms but may be omitted when
    /// authenticating via MONGODB-X509.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// The database used to authenticate. This applies to all mechanisms and defaults to "admin"
    /// in SCRAM authentication mechanisms, "$external" for GSSAPI and MONGODB-X509, and the
    /// database name or "$external" for PLAIN.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// The password to authenticate with. This does not apply to all mechanisms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// Which authentication mechanism to use. If not provided, one will be negotiated with the
    /// server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mechanism: Option<AuthMechanism>,

    /// Additional properties for the given mechanism.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mechanism_properties: Option<Document>,
}

#[allow(missing_docs)]
impl Credential {
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username.replace(username.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source.replace(source.into());
        self
    }

    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password.replace(password.into());
        self
    }

    pub fn with_mechanism(mut self, mechanism: impl Into<AuthMechanism>) -> Self {
        self.mechanism.replace(mechanism.into());
        self
    }

    pub fn with_mechanism_properties(mut self, properties: impl Into<Document>) -> Self {
        self.mechanism_properties.replace(properties.into());
        self
    }
}

impl From<Credential> for mongodb::options::Credential {
    fn from(credential: Credential) -> Self {
        Self::builder()
            .username(credential.username)
            .source(credential.source)
            .password(credential.password)
            .mechanism(credential.mechanism.map(Into::into))
            .mechanism_properties(credential.mechanism_properties)
            .build()
    }
}

impl From<mongodb::options::Credential> for Credential {
    fn from(credential: mongodb::options::Credential) -> Self {
        Credential {
            username: credential.username,
            source: credential.source,
            password: credential.password,
            mechanism: credential.mechanism.map(Into::into),
            mechanism_properties: credential.mechanism_properties,
        }
    }
}

/// The authentication mechanisms supported by MongoDB.
///
/// Note: not all of these mechanisms are currently supported by the driver.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum AuthMechanism {
    /// MongoDB Challenge Response nonce and MD5 based authentication system. It is currently
    /// deprecated and will never be supported by this driver.
    MongoDbCr,

    /// The SCRAM-SHA-1 mechanism as defined in [RFC 5802](http://tools.ietf.org/html/rfc5802).
    ///
    /// See the [MongoDB documentation](https://docs.mongodb.com/manual/core/security-scram/) for more information.
    ScramSha1,

    /// The SCRAM-SHA-256 mechanism which extends [RFC 5802](http://tools.ietf.org/html/rfc5802) and is formally defined in [RFC 7677](https://tools.ietf.org/html/rfc7677).
    ///
    /// See the [MongoDB documentation](https://docs.mongodb.com/manual/core/security-scram/) for more information.
    ScramSha256,

    /// The MONGODB-X509 mechanism based on the usage of X.509 certificates to validate a client
    /// where the distinguished subject name of the client certificate acts as the username.
    ///
    /// See the [MongoDB documentation](https://docs.mongodb.com/manual/core/security-x.509/) for more information.
    MongoDbX509,

    /// Kerberos authentication mechanism as defined in [RFC 4752](http://tools.ietf.org/html/rfc4752).
    ///
    /// See the [MongoDB documentation](https://docs.mongodb.com/manual/core/kerberos/) for more information.
    ///
    /// Note: This mechanism is not currently supported by this driver but will be in the future.
    Gssapi,

    /// The SASL PLAIN mechanism, as defined in [RFC 4616](), is used in MongoDB to perform LDAP
    /// authentication and cannot be used for any other type of authentication.
    /// Since the credentials are stored outside of MongoDB, the "$external" database must be used
    /// for authentication.
    ///
    /// See the [MongoDB documentation](https://docs.mongodb.com/manual/core/security-ldap/#ldap-proxy-authentication) for more information on LDAP authentication.
    Plain,

    /// MONGODB-AWS authenticates using AWS IAM credentials (an access key ID and a secret access
    /// key), temporary AWS IAM credentials obtained from an AWS Security Token Service (STS)
    /// Assume Role request, or temporary AWS IAM credentials assigned to an EC2 instance or ECS
    /// task.
    ///
    /// Note: Only server versions 4.4+ support AWS authentication. Additionally, the driver only
    /// supports AWS authentication with the tokio runtime.
    MongoDbAws,
}

impl From<AuthMechanism> for mongodb::options::AuthMechanism {
    fn from(mechanism: AuthMechanism) -> Self {
        match mechanism {
            AuthMechanism::MongoDbCr => Self::MongoDbCr,
            AuthMechanism::ScramSha1 => Self::ScramSha1,
            AuthMechanism::ScramSha256 => Self::ScramSha256,
            AuthMechanism::MongoDbX509 => Self::MongoDbX509,
            AuthMechanism::Gssapi => Self::Gssapi,
            AuthMechanism::Plain => Self::Plain,
            AuthMechanism::MongoDbAws => {
                cfg_if! {
                    if #[cfg(feature = "mongodb/aws-auth")] {
                        Self::MongoDbAws
                    } else {
                        panic!("mongodb/aws-auth feature flag not enabled")
                    }
                }
            }
        }
    }
}

impl From<mongodb::options::AuthMechanism> for AuthMechanism {
    fn from(mechanism: mongodb::options::AuthMechanism) -> Self {
        match mechanism {
            mongodb::options::AuthMechanism::MongoDbCr => AuthMechanism::MongoDbCr,
            mongodb::options::AuthMechanism::ScramSha1 => AuthMechanism::ScramSha1,
            mongodb::options::AuthMechanism::ScramSha256 => AuthMechanism::ScramSha256,
            mongodb::options::AuthMechanism::MongoDbX509 => AuthMechanism::MongoDbX509,
            mongodb::options::AuthMechanism::Gssapi => AuthMechanism::Gssapi,
            mongodb::options::AuthMechanism::Plain => AuthMechanism::Plain,
            #[cfg(feature = "mongodb/aws-auth")]
            mongodb::options::AuthMechanism::MongoDbAws => AuthMechanism::MongoDbAws,
            _ => panic!("Unsupported authentication mechanism"),
        }
    }
}
