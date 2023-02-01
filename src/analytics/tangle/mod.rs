// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the tangle.

pub(crate) use self::{block_activity::BlockActivityMeasurement, milestone_size::MilestoneSizeMeasurement};
use crate::{
    analytics::{influx::PerMilestone, Analytics},
    tangle::BlockData,
    types::{ledger::MilestoneIndexTimestamp, stardust::block::Payload, tangle::ProtocolParameters},
};

mod block_activity;
mod milestone_size;
