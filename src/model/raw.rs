// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use packable::{Packable, PackableExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct InvalidRawBytesError(pub String);

/// Represents a type as raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw<T: Packable> {
    data: Vec<u8>,
    _phantom: PhantomData<T>,
}

impl<T: Packable> Raw<T> {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            data: bytes.into(),
            _phantom: PhantomData,
        }
    }

    /// Retrieves the underlying raw data.
    #[must_use]
    pub fn data(self) -> Vec<u8> {
        self.data
    }

    /// Unpack the raw data into a type `T` using
    /// [`ProtocolParameters`](iota_sdk::types::block::protocol::ProtocolParameters) to verify the bytes.
    pub fn inner(self, visitor: &T::UnpackVisitor) -> Result<T, InvalidRawBytesError> {
        let unpacked = T::unpack_verified(self.data, visitor).map_err(|e| InvalidRawBytesError(format!("{e:?}")))?;
        Ok(unpacked)
    }

    /// Unpack the raw data into a type `T` without performing syntactic or semantic validation. This is useful if the
    /// type is guaranteed to be well-formed, for example when it was transmitted via the INX interface.
    pub fn inner_unverified(self) -> Result<T, InvalidRawBytesError> {
        let unpacked = T::unpack_unverified(self.data).map_err(|e| InvalidRawBytesError(format!("{e:?}")))?;
        Ok(unpacked)
    }
}

impl<T: Packable> From<T> for Raw<T> {
    fn from(value: T) -> Self {
        Self::from_bytes(value.pack_to_vec())
    }
}

impl<T: Packable> Serialize for Raw<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_bytes::serialize(&self.data, serializer)
    }
}

impl<'de, T: Packable> Deserialize<'de> for Raw<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde_bytes::deserialize::<Vec<u8>, _>(deserializer).map(Raw::from_bytes)
    }
}
