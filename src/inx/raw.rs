// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use inx::proto;
use iota_sdk::types::block::{output::Output, payload::Payload, slot::SlotCommitment, SignedBlock};
use packable::{Packable, PackableExt};

use super::InxError;

/// Represents a type as raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw<T: Packable> {
    data: Vec<u8>,
    _phantom: PhantomData<T>,
}

impl<T: Packable> Raw<T> {
    /// Retrieves the underlying raw data.
    #[must_use]
    pub fn data(self) -> Vec<u8> {
        self.data
    }

    /// Unpack the raw data into a type `T` using
    /// [`ProtocolParameters`](iota_sdk::types::block::protocol::ProtocolParameters) to verify the bytes.
    pub fn inner(self, visitor: &T::UnpackVisitor) -> Result<T, InxError> {
        let unpacked =
            T::unpack_verified(self.data, visitor).map_err(|e| InxError::InvalidRawBytes(format!("{e:?}")))?;
        Ok(unpacked)
    }

    /// Unpack the raw data into a type `T` without performing syntactic or semantic validation. This is useful if the
    /// type is guaranteed to be well-formed, for example when it was transmitted via the INX interface.
    pub fn inner_unverified(self) -> Result<T, InxError> {
        let unpacked = T::unpack_unverified(self.data).map_err(|e| InxError::InvalidRawBytes(format!("{e:?}")))?;
        Ok(unpacked)
    }
}

impl<T: Packable> From<Vec<u8>> for Raw<T> {
    fn from(value: Vec<u8>) -> Self {
        Self {
            data: value,
            _phantom: PhantomData,
        }
    }
}

impl From<proto::RawOutput> for Raw<Output> {
    fn from(value: proto::RawOutput) -> Self {
        value.data.into()
    }
}

impl From<proto::RawBlock> for Raw<SignedBlock> {
    fn from(value: proto::RawBlock) -> Self {
        value.data.into()
    }
}

impl From<proto::RawPayload> for Raw<Payload> {
    fn from(value: proto::RawPayload) -> Self {
        value.data.into()
    }
}

impl From<proto::RawCommitment> for Raw<SlotCommitment> {
    fn from(value: proto::RawCommitment) -> Self {
        value.data.into()
    }
}
