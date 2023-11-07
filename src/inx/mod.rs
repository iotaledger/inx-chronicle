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
use crate::model::raw::Raw;

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
