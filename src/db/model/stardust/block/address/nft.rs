// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::address as bee;
use serde::{Deserialize, Serialize};

use crate::{db, db::model::stardust::block::NftId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    type Err = db::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::NftAddress::from_str(s)?.into())
    }
}
