// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::ByteCostConfig;
use serde::{Deserialize, Serialize};

/// Parameters relevant to byte cost calculations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RentStructure {
    pub v_byte_cost: u32,
    pub v_byte_factor_data: u32,
    pub v_byte_factor_key: u32,
}

impl From<ByteCostConfig> for RentStructure {
    fn from(value: ByteCostConfig) -> Self {
        Self {
            v_byte_cost: value.v_byte_cost as u32,
            v_byte_factor_data: value.v_byte_factor_data as u32,
            v_byte_factor_key: value.v_byte_factor_key as u32,
        }
    }
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

impl From<inx::ProtocolParameters> for ProtocolParameters {
    fn from(value: inx::ProtocolParameters) -> Self {
        Self {
            version: value.version,
            network_name: value.network_name,
            bech32_hrp: value.bech32_hrp,
            min_pow_score: value.min_pow_score,
            below_max_depth: value.below_max_depth,
            rent_structure: value.rent_structure.into(),
            token_supply: value.token_supply,
        }
    }
}
