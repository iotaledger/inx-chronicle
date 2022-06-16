// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct CliArgs {
    /// The location of the configuration file.
    #[clap(short, value_parser, long)]
    pub config: Option<String>,
    /// The address of the INX interface provided by the node.
    #[clap(value_parser, long)]
    pub inx: Option<String>,
    /// The address of the MongoDB database.
    #[clap(value_parser, long)]
    pub db: Option<String>,
}
