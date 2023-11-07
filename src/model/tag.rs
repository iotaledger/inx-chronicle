// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::str::FromStr;

use serde::Deserialize;

use super::*;

/// A [`Tag`] associated with an [`Output`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(#[serde(with = "serde_bytes")] Vec<u8>);

impl Tag {
    /// Creates a [`Tag`] from bytes.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// Creates a [`Tag`] from `0x`-prefixed hex representation.
    pub fn from_hex<T: AsRef<str>>(tag: T) -> Result<Self, prefix_hex::Error> {
        Ok(Self(prefix_hex::decode::<Vec<u8>>(tag.as_ref())?))
    }

    /// Converts the [`Tag`] to its `0x`-prefixed hex representation.
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(&*self.0)
    }
}

// Note: assumes an ASCII string as input.
impl<T: ToString> From<T> for Tag {
    fn from(value: T) -> Self {
        Self(value.to_string().into_bytes())
    }
}

// Note: assumes a `0x`-prefixed hex representation as input.
impl FromStr for Tag {
    type Err = prefix_hex::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

impl From<Tag> for Bson {
    fn from(val: Tag) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&serde_bytes::ByteBuf::from(val.0)).unwrap()
    }
}
