// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::address as bee;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::output::AliasId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AliasAddress(pub AliasId);

impl From<bee::AliasAddress> for AliasAddress {
    fn from(value: bee::AliasAddress) -> Self {
        Self((*value).into())
    }
}

impl From<AliasAddress> for bee::AliasAddress {
    fn from(value: AliasAddress) -> Self {
        bee::AliasAddress::new(value.0.into())
    }
}

impl FromStr for AliasAddress {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::AliasAddress::from_str(s)?.into())
    }
}

impl From<AliasAddress> for Bson {
    fn from(val: AliasAddress) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&val).unwrap()
    }
}
