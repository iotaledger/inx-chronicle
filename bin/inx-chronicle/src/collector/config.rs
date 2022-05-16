// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub solidifier_count: usize,
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub inx: super::stardust_inx::InxConfig,
}

impl CollectorConfig {
    const MAX_SOLIDIFIERS: usize = 100;

    pub fn new(solidifier_count: usize, #[cfg(all(feature = "stardust", feature = "inx"))] inx: String) -> Self {
        Self {
            solidifier_count: solidifier_count.clamp(1, Self::MAX_SOLIDIFIERS),
            inx: super::stardust_inx::InxConfig::new(inx),
        }
    }
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self::new(
            10,
            #[cfg(all(feature = "stardust", feature = "inx"))]
            Default::default(),
        )
    }
}
