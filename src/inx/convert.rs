// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use inx::proto;
use iota_sdk::types::block::{
    output::OutputId, payload::signed_transaction::TransactionId, slot::SlotCommitmentId, BlockId,
};

use super::InxError;
use crate::model::raw::InvalidRawBytesError;

/// Tries to access the field of a protobug messages and returns an appropriate error if the field is not present.
#[macro_export]
macro_rules! maybe_missing {
    ($object:ident.$field:ident) => {
        $object
            .$field
            .ok_or($crate::inx::InxError::MissingField(stringify!($field)))?
    };
}

pub(crate) trait ConvertTo<T> {
    fn convert(self) -> T;
}

impl<T, U> ConvertTo<T> for U
where
    T: ConvertFrom<U>,
{
    fn convert(self) -> T {
        T::convert_from(self)
    }
}

pub(crate) trait ConvertFrom<P> {
    fn convert_from(proto: P) -> Self
    where
        Self: Sized;
}

impl<R: ConvertTo<U>, U> ConvertFrom<inx::tonic::Response<R>> for U {
    fn convert_from(proto: inx::tonic::Response<R>) -> Self
    where
        Self: Sized,
    {
        proto.into_inner().convert()
    }
}

pub(crate) trait TryConvertTo<T> {
    type Error;

    fn try_convert(self) -> Result<T, Self::Error>;
}

impl<T, U> TryConvertTo<T> for U
where
    T: TryConvertFrom<U>,
{
    type Error = T::Error;

    fn try_convert(self) -> Result<T, Self::Error> {
        T::try_convert_from(self)
    }
}

pub(crate) trait TryConvertFrom<P> {
    type Error;

    fn try_convert_from(proto: P) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

impl<R: TryConvertTo<U>, U> TryConvertFrom<inx::tonic::Response<R>> for U {
    type Error = R::Error;

    fn try_convert_from(proto: inx::tonic::Response<R>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        proto.into_inner().try_convert()
    }
}

impl TryConvertFrom<proto::BlockId> for BlockId {
    type Error = InvalidRawBytesError;

    fn try_convert_from(proto: proto::BlockId) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self::new(proto.id.try_into().map_err(|e| {
            InvalidRawBytesError(format!("invalid block id bytes: {}", hex::encode(e)))
        })?))
    }
}

impl TryConvertFrom<proto::TransactionId> for TransactionId {
    type Error = InvalidRawBytesError;

    fn try_convert_from(proto: proto::TransactionId) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self::new(proto.id.try_into().map_err(|e| {
            InvalidRawBytesError(format!("invalid transaction id bytes: {}", hex::encode(e)))
        })?))
    }
}

impl TryConvertFrom<proto::CommitmentId> for SlotCommitmentId {
    type Error = InvalidRawBytesError;

    fn try_convert_from(proto: proto::CommitmentId) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self::new(proto.id.try_into().map_err(|e| {
            InvalidRawBytesError(format!("invalid commitment id bytes: {}", hex::encode(e)))
        })?))
    }
}

impl TryConvertFrom<proto::OutputId> for OutputId {
    type Error = InxError;

    fn try_convert_from(proto: proto::OutputId) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self::try_from(<[u8; Self::LENGTH]>::try_from(proto.id).map_err(
            |e| InvalidRawBytesError(format!("invalid output id bytes: {}", hex::encode(e))),
        )?)?)
    }
}
