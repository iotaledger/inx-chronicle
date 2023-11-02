// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing convenience wrappers around the low-level [`INX`](inx) bindings.

// mod block;
/// The INX client.
pub mod client;
mod convert;
mod error;
/// Types for the ledger.
pub mod ledger;
pub mod responses;
// mod node;
/// Raw message helper types;
pub mod raw;
mod request;

pub use self::error::InxError;
