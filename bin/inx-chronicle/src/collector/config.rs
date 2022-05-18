// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CollectorConfig {
    #[cfg(all(feature = "stardust", feature = "inx"))]
    pub inx: super::stardust_inx::InxConfig,
}
