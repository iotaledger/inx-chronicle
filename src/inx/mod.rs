// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing convenience wrappers around the low-level [`INX`](inx) bindings.

/// The INX client.
pub mod client;
mod convert;
mod error;
/// Types for the ledger.
pub mod ledger;
mod request;
pub mod responses;

use inx::proto;
use iota_sdk::types::block::{output::Output, payload::Payload, slot::SlotCommitment, SignedBlock};

pub use self::{client::Inx, error::InxError, request::SlotRangeRequest};
use crate::model::raw::{InvalidRawBytesError, Raw};

impl TryFrom<proto::RawOutput> for Raw<Output> {
    type Error = InvalidRawBytesError;

    fn try_from(value: proto::RawOutput) -> Result<Self, Self::Error> {
        Raw::from_bytes(value.data)
    }
}

impl TryFrom<proto::RawBlock> for Raw<SignedBlock> {
    type Error = InvalidRawBytesError;

    fn try_from(value: proto::RawBlock) -> Result<Self, Self::Error> {
        Raw::from_bytes(value.data)
    }
}

impl TryFrom<proto::RawPayload> for Raw<Payload> {
    type Error = InvalidRawBytesError;

    fn try_from(value: proto::RawPayload) -> Result<Self, Self::Error> {
        Raw::from_bytes(value.data)
    }
}

impl TryFrom<proto::RawCommitment> for Raw<SlotCommitment> {
    type Error = InvalidRawBytesError;

    fn try_from(value: proto::RawCommitment) -> Result<Self, Self::Error> {
        Raw::from_bytes(value.data)
    }
}
