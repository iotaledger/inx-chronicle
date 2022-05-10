// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[cfg(all(feature = "stardust", feature = "inx"))]
use super::stardust_inx::StardustInxConfig;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub solidifier_count: usize,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub inx: StardustInxConfig,
}

impl CollectorConfig {
    const MAX_SOLIDIFIERS: usize = 100;

    pub fn new(solidifier_count: usize, inx_config: StardustInxConfig) -> Self {
        Self {
            solidifier_count: solidifier_count.clamp(1, Self::MAX_SOLIDIFIERS),
            inx: inx_config,
        }
    }
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self::new(10, Default::default())
    }
}
