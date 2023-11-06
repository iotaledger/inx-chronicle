// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing convenience wrappers around the low-level [`INX`](inx) bindings.

/// The INX client.
pub mod client;
mod convert;
mod error;
/// Types for the ledger.
pub mod ledger;
/// Raw message helper types;
pub mod raw;
mod request;
pub mod responses;

pub use self::{client::Inx, error::InxError, request::SlotRangeRequest};
