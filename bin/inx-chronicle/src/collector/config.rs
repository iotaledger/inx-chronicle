// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CollectorConfig {
    pub solidifier_count: usize,
}

impl CollectorConfig {
    const MAX_SOLIDIFIERS: usize = 100;

    pub fn new(solidifier_count: usize) -> Self {
        Self {
            solidifier_count: solidifier_count.clamp(1, Self::MAX_SOLIDIFIERS),
        }
    }
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self::new(10)
    }
}
