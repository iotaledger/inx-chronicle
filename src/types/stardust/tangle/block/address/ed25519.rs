// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_types::block::address as iota;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use crate::types::serde::bytify;

/// A regular Ed25519 address.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct Ed25519Address(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl Ed25519Address {
    const LENGTH: usize = iota::Ed25519Address::LENGTH;
}

impl From<iota::Ed25519Address> for Ed25519Address {
    fn from(value: iota::Ed25519Address) -> Self {
        Self(*value)
    }
}

impl From<Ed25519Address> for iota::Ed25519Address {
    fn from(value: Ed25519Address) -> Self {
        iota::Ed25519Address::new(value.0)
    }
}

impl From<Ed25519Address> for iota::dto::Ed25519AddressDto {
    fn from(value: Ed25519Address) -> Self {
        Into::into(&iota::Ed25519Address::from(value))
    }
}

impl FromStr for Ed25519Address {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::Ed25519Address::from_str(s)?.into())
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
    use iota_types::block::rand::address::rand_ed25519_address;

    use super::*;

    impl Ed25519Address {
        /// Generates a random [`Ed25519Address`].
        pub fn rand() -> Self {
            rand_ed25519_address().into()
        }
    }
}
