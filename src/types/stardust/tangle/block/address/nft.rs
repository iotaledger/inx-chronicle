// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_types::block::address as iota;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

use crate::types::stardust::tangle::block::output::NftId;

/// An NFT address.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct NftAddress(pub NftId);

impl From<iota::NftAddress> for NftAddress {
    fn from(value: iota::NftAddress) -> Self {
        Self((*value).into())
    }
}

impl From<NftAddress> for iota::NftAddress {
    fn from(value: NftAddress) -> Self {
        iota::NftAddress::new(value.0.into())
    }
}

impl From<NftAddress> for iota::dto::NftAddressDto {
    fn from(value: NftAddress) -> Self {
        Into::into(&iota::NftAddress::from(value))
    }
}

impl FromStr for NftAddress {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::NftAddress::from_str(s)?.into())
    }
}

impl From<NftAddress> for Bson {
    fn from(val: NftAddress) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&val).unwrap()
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::address::rand_nft_address;

    use super::*;

    impl NftAddress {
        /// Generates a random [`NftAddress`].
        pub fn rand() -> Self {
            rand_nft_address().into()
        }
    }
}
