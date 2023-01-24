// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::TransactionAnalytics;
use crate::types::{stardust::block::Output, tangle::MilestoneIndex};

#[derive(Clone, Debug, Default)]
pub struct BaseTokenActivity {
    pub booked_value: u64,
    pub transferred_value: u64,
}

struct BaseTokenActivityAnalytics {
    measurement: BaseTokenActivity,
}

impl TransactionAnalytics for BaseTokenActivityAnalytics {
    type Measurement = BaseTokenActivity;

    fn begin_milestone(&mut self, _: MilestoneIndex) {
        self.measurement = BaseTokenActivity::default();
    }

    fn handle_transaction(&mut self, inputs: &[Output], outputs: &[Output]) {
        todo!()
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement.clone())
    }
}
