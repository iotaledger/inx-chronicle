// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the tangle models.

pub mod context;
pub mod protocol;

pub use self::{
    context::*,
    protocol::{ProtocolParameters, RentStructure},
};
