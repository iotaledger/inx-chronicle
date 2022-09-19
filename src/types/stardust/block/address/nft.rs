// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::address as bee;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::output::NftId;

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

impl FromStr for NftAddress {
    type Err = bee_block_stardust::Error;

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
    use bee_block_stardust::rand::address::rand_nft_address;

    use super::*;

    impl NftAddress {
        /// Generates a random [`NftAddress`].
        pub fn rand() -> Self {
            rand_nft_address().into()
        }
    }
}
