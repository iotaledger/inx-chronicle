// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

/// Chronicle permanode storage as an INX plugin
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct CliArgs {
    /// Location of the configuration file
    #[clap(short, long)]
    pub config: Option<String>,
}
