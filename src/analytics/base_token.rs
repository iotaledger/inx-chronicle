// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::TransactionAnalytics;
use crate::types::{stardust::block::{Output, Address}, tangle::MilestoneIndex};

/// Measures activity of the base token, such as Shimmer or IOTA.
#[derive(Clone, Debug, Default)]
pub struct BaseTokenActivity {
    /// Represents the amount of tokens transfered. Tokens that are send back to an address are not counted.
    pub booked_value: usize,
    /// Represents the total amount of tokens transfered, independent of wether tokens were sent back to same address.
    pub transferred_value: usize,
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
        let mut outflows: HashMap<&Address, usize> = HashMap::new();
        for input in inputs {
            if let Some(address) = input.owning_address() {
                self.measurement.transferred_value += input.amount().0 as usize;
                *outflows.entry(address).or_default() += input.amount().0 as usize;
            }
        }
        for output in outputs {
            if let Some(address) = output.owning_address() {
                if let Some(entry) = outflows.get_mut(address) {
                    *entry -= output.amount().0 as usize;
                }
            }
        }
        self.measurement.booked_value = outflows.values().sum();
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement.clone())
    }
}
