// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::address as bee;
use serde::{Deserialize, Serialize};

use crate::{db, db::model::util::bytify};


#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

impl FromStr for Ed25519Address {
    type Err = db::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::Ed25519Address::from_str(s)?.into())
    }
}
