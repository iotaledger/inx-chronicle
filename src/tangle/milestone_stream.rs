// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::{stardust::block::payload::{MilestoneId, MilestonePayload}, ledger::MilestoneIndexTimestamp, tangle::ProtocolParameters};

#[allow(missing_docs)]
pub struct MilestoneAndProtocolParameters {
    pub milestone_id: MilestoneId,
    pub at: MilestoneIndexTimestamp,
    pub payload: MilestonePayload,
    pub protocol_params: ProtocolParameters,
}
