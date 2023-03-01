// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the protocol models.

#![allow(missing_docs)]

mod context;

use iota_types::block as iota;
use serde::{Deserialize, Serialize};

pub use self::context::*;
use crate::model::serde::stringify;

/// Parameters relevant to byte cost calculations.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RentStructure {
    pub v_byte_cost: u32,
    pub v_byte_factor_data: u8,
    pub v_byte_factor_key: u8,
}

impl From<&iota::output::RentStructure> for RentStructure {
    fn from(value: &iota::output::RentStructure) -> Self {
        Self {
            v_byte_cost: value.byte_cost(),
            v_byte_factor_data: value.byte_factor_data(),
            v_byte_factor_key: value.byte_factor_key(),
        }
    }
}

impl From<RentStructure> for iota::output::RentStructure {
    fn from(value: RentStructure) -> Self {
        Self::build()
            .byte_cost(value.v_byte_cost)
            .byte_factor_data(value.v_byte_factor_data)
            .byte_factor_key(value.v_byte_factor_key)
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
    #[serde(with = "stringify")]
    pub token_supply: u64,
}

impl From<iota::protocol::ProtocolParameters> for ProtocolParameters {
    fn from(value: iota::protocol::ProtocolParameters) -> Self {
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

impl TryFrom<ProtocolParameters> for iota::protocol::ProtocolParameters {
    type Error = iota_types::block::Error;

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
