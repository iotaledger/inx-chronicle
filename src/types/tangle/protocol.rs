// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use bee_block_stardust as bee;
use serde::{Deserialize, Serialize};

/// Parameters relevant to byte cost calculations.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RentStructure {
    pub v_byte_cost: u32,
    pub v_byte_factor_data: u8,
    pub v_byte_factor_key: u8,
}

impl From<&bee::output::RentStructure> for RentStructure {
    fn from(value: &bee::output::RentStructure) -> Self {
        Self {
            v_byte_cost: value.v_byte_cost,
            v_byte_factor_data: value.v_byte_factor_data,
            v_byte_factor_key: value.v_byte_factor_key,
        }
    }
}

impl From<RentStructure> for bee::output::RentStructure {
    fn from(value: RentStructure) -> Self {
        Self::build()
            .byte_cost(value.v_byte_cost)
            .data_factor(value.v_byte_factor_data)
            .key_factor(value.v_byte_factor_key)
            .finish()
    }
}

/// Protocol parameters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolParameters {
    pub version: u8,
    pub network_name: String,
    pub bech32_hrp: String,
    pub min_pow_score: u32,
    pub below_max_depth: u8,
    pub rent_structure: RentStructure,
    #[serde(with = "crate::types::util::stringify")]
    pub token_supply: u64,
}

impl From<bee::protocol::ProtocolParameters> for ProtocolParameters {
    fn from(value: bee::protocol::ProtocolParameters) -> Self {
        Self {
            version: value.protocol_version(),
            network_name: value.network_name().into(),
            bech32_hrp: value.bech32_hrp().into(),
            min_pow_score: value.min_pow_score(),
            below_max_depth: value.below_max_depth(),
            rent_structure: value.rent_structure().into(),
            token_supply: value.token_supply(),
        }
    }
}

impl TryFrom<ProtocolParameters> for bee::protocol::ProtocolParameters {
    type Error = bee_block_stardust::Error;

    fn try_from(value: ProtocolParameters) -> Result<Self, Self::Error> {
        Self::new(
            value.version,
            value.network_name,
            value.bech32_hrp,
            value.min_pow_score,
            value.below_max_depth,
            value.rent_structure.into(),
            value.token_supply,
        )
    }
}
