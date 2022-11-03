// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the tangle models.

mod milestone;
mod protocol;

pub use self::{
    milestone::MilestoneIndex,
    protocol::{ProtocolParameters, RentStructure},
};
