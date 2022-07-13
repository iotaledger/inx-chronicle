// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// Parameters relevant to byte cost calculations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RentStructure {
    pub v_byte_cost: u32,
    pub v_byte_factor_data: u32,
    pub v_byte_factor_key: u32,
}

/// Protocol parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolParameters {
    pub version: u32,
    pub network_name: String,
    pub bech32_hrp: String,
    pub min_pow_score: u32,
    pub below_max_depth: u32,
    pub rent_structure: RentStructure,
    #[serde(with = "crate::types::util::stringify")]
    pub token_supply: u64,
}
