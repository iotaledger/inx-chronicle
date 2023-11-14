// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the raw bytes helper type.

use packable::{Packable, PackableExt};
use serde::{Deserialize, Serialize};

/// An error that indicates that raw bytes were invalid.
#[derive(Debug, thiserror::Error)]
#[error("invalid raw bytes: {0}")]
pub struct InvalidRawBytesError(pub String);

/// Represents a type as raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw<T: Packable> {
    data: Vec<u8>,
    inner: T,
}

impl<T: Packable> Raw<T> {
    /// Create a raw value from bytes.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, InvalidRawBytesError> {
        let data = bytes.into();
        Ok(Self {
            inner: T::unpack_unverified(&data)
                .map_err(|e| InvalidRawBytesError(format!("error unpacking {}: {e:?}", std::any::type_name::<T>())))?,
            data,
        })
    }

    /// Retrieves the underlying raw data.
    #[must_use]
    pub fn data(self) -> Vec<u8> {
        self.data
    }

    /// Get the inner value.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Consume the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Packable> From<T> for Raw<T> {
    fn from(value: T) -> Self {
        Self {
            data: value.pack_to_vec(),
            inner: value,
        }
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
        serde_bytes::deserialize::<Vec<u8>, _>(deserializer)
            .and_then(|bytes| Self::from_bytes(bytes).map_err(serde::de::Error::custom))
    }
}
