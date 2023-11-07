// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use inx::proto;
use iota_sdk::types::block::{
    output::{Output, OutputId},
    payload::{signed_transaction::TransactionId, Payload},
    slot::{SlotCommitment, SlotCommitmentId},
    BlockId, SignedBlock,
};

use super::InxError;
use crate::model::raw::{InvalidRawBytesError, Raw};

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

macro_rules! impl_id_convert {
    ($type:ident) => {
        impl TryConvertFrom<proto::$type> for $type {
            type Error = InvalidRawBytesError;

            fn try_convert_from(proto: proto::$type) -> Result<Self, Self::Error>
            where
                Self: Sized,
            {
                Ok(Self::new(proto.id.try_into().map_err(|e| {
                    InvalidRawBytesError(format!("{}", hex::encode(e)))
                })?))
            }
        }
    };
}
impl_id_convert!(BlockId);
impl_id_convert!(TransactionId);

impl TryConvertFrom<proto::CommitmentId> for SlotCommitmentId {
    type Error = InvalidRawBytesError;

    fn try_convert_from(proto: proto::CommitmentId) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self::new(
            proto
                .id
                .try_into()
                .map_err(|e| InvalidRawBytesError(format!("{}", hex::encode(e))))?,
        ))
    }
}

impl TryConvertFrom<proto::OutputId> for OutputId {
    type Error = InxError;

    fn try_convert_from(proto: proto::OutputId) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self::try_from(
            <[u8; Self::LENGTH]>::try_from(proto.id)
                .map_err(|e| InvalidRawBytesError(format!("{}", hex::encode(e))))?,
        )?)
    }
}

macro_rules! impl_raw_convert {
    ($raw:ident, $type:ident) => {
        impl TryConvertFrom<proto::$raw> for $type {
            type Error = InvalidRawBytesError;

            fn try_convert_from(proto: proto::$raw) -> Result<Self, Self::Error>
            where
                Self: Sized,
            {
                Raw::from(proto).inner_unverified()
            }
        }
    };
}
impl_raw_convert!(RawOutput, Output);
impl_raw_convert!(RawBlock, SignedBlock);
impl_raw_convert!(RawPayload, Payload);
impl_raw_convert!(RawCommitment, SlotCommitment);
