// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use super::TransactionAnalytics;
use crate::types::{
    stardust::block::{Address, Output},
    tangle::MilestoneIndex,
};

pub struct AddressCount(usize);

struct AddressBalanceAnalytics {
    addresses: HashSet<Address>,
}

impl TransactionAnalytics for AddressBalanceAnalytics {
    type Measurement = AddressCount;

    fn begin_milestone(&mut self, _: MilestoneIndex) {}

    fn handle_transaction(&mut self, inputs: &[Output], outputs: &[Output]) {
        for input in inputs {
            if let Some(a) = input.owning_address() {
                self.addresses.remove(a);
            }
        }

        for output in outputs {
            if let Some(a) = output.owning_address() {
                self.addresses.insert(a.clone());
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(AddressCount(self.addresses.len()))
    }
}
