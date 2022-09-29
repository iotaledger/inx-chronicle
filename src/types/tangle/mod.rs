// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the tangle models.

mod milestone_index;
mod protocol;

pub use self::{
    milestone_index::MilestoneIndex,
    protocol::{ProtocolParameters, RentStructure},
};
