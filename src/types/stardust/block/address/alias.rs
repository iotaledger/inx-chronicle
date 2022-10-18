// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_types::block::address as iota;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::output::AliasId;

/// An address of an alias.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AliasAddress(pub AliasId);

impl From<iota::AliasAddress> for AliasAddress {
    fn from(value: iota::AliasAddress) -> Self {
        Self((*value).into())
    }
}

impl From<AliasAddress> for iota::AliasAddress {
    fn from(value: AliasAddress) -> Self {
        iota::AliasAddress::new(value.0.into())
    }
}

impl From<AliasAddress> for iota::dto::AliasAddressDto {
    fn from(value: AliasAddress) -> Self {
        Into::into(&iota::AliasAddress::from(value))
    }
}

impl FromStr for AliasAddress {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::AliasAddress::from_str(s)?.into())
    }
}

impl From<AliasAddress> for Bson {
    fn from(val: AliasAddress) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&val).unwrap()
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::address::rand_alias_address;

    use super::*;

    impl AliasAddress {
        /// Generates a random [`AliasAddress`].
        pub fn rand() -> Self {
            rand_alias_address().into()
        }
    }
}
