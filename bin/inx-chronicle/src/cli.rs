// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct ClArgs {
    /// The location of the configuration file.
    #[clap(short, long)]
    pub config: Option<String>,
    /// The address of the MongoDB database.
    #[clap(long = "db")]
    pub db_addr: Option<String>,
    /// The address of the INX interface provided by the node.
    #[clap(long = "inx")]
    pub inx_addr: Option<String>,
    /// Toggle INX (offline mode).
    #[clap(long, value_parser, env = "INX")]
    #[cfg(feature = "inx")]
    pub enable_inx: Option<bool>,
    /// The location of the identity file for JWT auth.
    #[clap(long)]
    pub identity: Option<String>,
    /// The password used for authentication.
    #[clap(long)]
    pub password: Option<String>,
    /// Toggle REST API.
    #[clap(long, value_parser, env = "API")]
    #[cfg(feature = "api")]
    pub enable_api: Option<bool>,
    /// Toggle the metrics server.
    #[clap(long, value_parser, env = "METRICS")]
    #[cfg(feature = "metrics")]
    pub enable_metrics: Option<bool>,
}
