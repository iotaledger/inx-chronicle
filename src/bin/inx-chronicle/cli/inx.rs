// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use clap::Args;

use crate::inx::config as inx;

#[derive(Args, Debug)]
pub struct InxArgs {
    /// The address of the node INX interface Chronicle tries to connect to - if enabled.
    #[arg(long, value_name = "URL", env = "INX_URL", default_value = inx::DEFAULT_URL)]
    pub inx_url: String,
    /// Milestone at which synchronization should begin. If set to `1` Chronicle will try to sync back until the
    /// genesis block. If set to `0` Chronicle will start syncing from the most recent milestone it received.
    #[arg(long, value_name = "START", default_value_t = inx::DEFAULT_SYNC_START)]
    pub inx_sync_start: u32,
    /// Disable the INX synchronization workflow.
    #[arg(long, default_value_t = !inx::DEFAULT_ENABLED)]
    pub disable_inx: bool,
}

impl From<&InxArgs> for inx::InxConfig {
    fn from(value: &InxArgs) -> Self {
        Self {
            enabled: !value.disable_inx,
            url: value.inx_url.clone(),
            sync_start_milestone: value.inx_sync_start.into(),
        }
    }
}
