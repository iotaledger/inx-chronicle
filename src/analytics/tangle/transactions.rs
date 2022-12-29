// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::TangleAnalytics;
use crate::types::ledger::LedgerInclusionState;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransactionStatistic {
    pub confirmed_count: u32,
    pub conflicting_count: u32,
    pub no_transaction_count: u32,
}

#[derive(Clone, Debug)]
pub struct TransactionAnalytics(TransactionStatistic);

impl TangleAnalytics for TransactionAnalytics {
    type Measurement = TransactionStatistic;

    fn begin(&mut self) {
        self.0 = Default::default()
    }

    fn handle_block(&mut self, msg: &crate::inx::BlockWithMetadataMessage) {
        match msg.metadata.inclusion_state {
            LedgerInclusionState::Included => self.0.confirmed_count += 1,
            LedgerInclusionState::Conflicting => self.0.conflicting_count += 1,
            LedgerInclusionState::NoTransaction => self.0.no_transaction_count += 1,
        }
    }

    fn flush(&mut self) -> Option<Self::Measurement> {
        Some(self.0.clone())
    }
}
