// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_types::block::address as bee;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::output::NftId;

/// An NFT address.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NftAddress(pub NftId);

impl From<bee::NftAddress> for NftAddress {
    fn from(value: bee::NftAddress) -> Self {
        Self((*value).into())
    }
}

impl From<NftAddress> for bee::NftAddress {
    fn from(value: NftAddress) -> Self {
        bee::NftAddress::new(value.0.into())
    }
}

impl From<NftAddress> for bee::dto::NftAddressDto {
    fn from(value: NftAddress) -> Self {
        Into::into(&bee::NftAddress::from(value))
    }
}

impl FromStr for NftAddress {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::NftAddress::from_str(s)?.into())
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
