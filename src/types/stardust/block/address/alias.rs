// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::address as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::AliasId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    type Err = crate::types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::AliasAddress::from_str(s)?.into())
    }
}
