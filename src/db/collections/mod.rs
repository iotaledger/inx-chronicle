// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing the Block document model.
mod block;
/// Module containing the Milestone document model.
mod milestone;
/// Module containing the Output document model.
mod output;
/// Module containing information about the network and state of the node.
mod status;
/// Module containing sync document models.
mod sync;

pub use self::{
    block::BlockDocument, milestone::MilestoneRecord, output::OutputDocument, status::StatusDocument,
    sync::SyncDocument,
};
