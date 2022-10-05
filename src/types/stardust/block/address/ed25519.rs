// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::address as bee;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use crate::types::util::bytify;

/// A regular Ed25519 address.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ed25519Address(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl Ed25519Address {
    const LENGTH: usize = bee::Ed25519Address::LENGTH;
}

impl From<bee::Ed25519Address> for Ed25519Address {
    fn from(value: bee::Ed25519Address) -> Self {
        Self(*value)
    }
}

impl From<Ed25519Address> for bee::Ed25519Address {
    fn from(value: Ed25519Address) -> Self {
        bee::Ed25519Address::new(value.0)
    }
}

impl From<Ed25519Address> for bee::dto::Ed25519AddressDto {
    fn from(value: Ed25519Address) -> Self {
        Into::into(&bee::Ed25519Address::from(value))
    }
}

impl FromStr for Ed25519Address {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::Ed25519Address::from_str(s)?.into())
    }
}

impl From<Ed25519Address> for Bson {
    fn from(val: Ed25519Address) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::address::rand_ed25519_address;

    use super::*;

    impl Ed25519Address {
        /// Generates a random [`Ed25519Address`].
        pub fn rand() -> Self {
            rand_ed25519_address().into()
        }
    }
}
