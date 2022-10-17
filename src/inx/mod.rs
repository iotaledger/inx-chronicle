// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing convenience wrappers around the low-level [`INX`](inx) bindings.

mod block;
mod client;
mod error;
mod id;
mod ledger;
mod milestone;
mod node;
mod protocol;
mod raw;
mod request;

pub use self::{
    block::{BlockMessage, BlockMetadataMessage, BlockWithMetadataMessage},
    client::Inx,
    error::InxError,
    ledger::{LedgerUpdateMessage, Marker},
    node::NodeStatusMessage,
    protocol::RawProtocolParametersMessage,
    raw::RawMessage,
    request::MilestoneRangeRequest,
};

/// Tries to access the field of a protobug messages and returns an appropriate error if the field is not present.
#[macro_export]
macro_rules! maybe_missing {
    ($object:ident.$field:ident) => {
        $object
            .$field
            .ok_or($crate::inx::InxError::MissingField(stringify!($field)))?
    };
}
