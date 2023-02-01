// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{influx::PerMilestone, Analytics};
use crate::types::{ledger::MilestoneIndexTimestamp, tangle::ProtocolParameters};

#[derive(Clone, Debug, Default)]
pub(crate) struct ProtocolParamsMeasurement {
    params: Option<ProtocolParameters>,
    updated: bool,
}

impl Analytics for ProtocolParamsMeasurement {
    type Measurement = PerMilestone<ProtocolParameters>;

    fn begin_milestone(&mut self, _at: MilestoneIndexTimestamp, params: &ProtocolParameters) {
        if matches!(&self.params, Some(last_params) if last_params == params) {
            self.updated = false;
        } else {
            self.params.replace(params.clone());
            self.updated = true;
        }
    }

    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        if self.updated {
            self.params.clone().map(|m| PerMilestone { at, inner: m })
        } else {
            None
        }
    }
}
