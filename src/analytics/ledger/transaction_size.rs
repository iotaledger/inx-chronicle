// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct TransactionSizeBuckets {
    pub(crate) single: [usize; 7], // 1,..,7
    pub(crate) small: usize,       // [8..16)
    pub(crate) medium: usize,      // [16..32)
    pub(crate) large: usize,       // [32..64)
    pub(crate) huge: usize,        // [64..128)
}

impl TransactionSizeBuckets {
    fn add(&mut self, value: usize) {
        match value {
            0 => unreachable!("invalid transaction"),
            1..=7 => self.single[value - 1] += 1,
            8..=15 => self.small += 1,
            16..=31 => self.medium += 1,
            32..=63 => self.large += 1,
            _ => self.huge += 1,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct TransactionSizeMeasurement {
    pub(crate) input_buckets: TransactionSizeBuckets,
    pub(crate) output_buckets: TransactionSizeBuckets,
}

impl Analytics for TransactionSizeMeasurement {
    type Measurement = TransactionSizeMeasurement;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {
        *self = Default::default()
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        self.input_buckets.add(consumed.len().into());
        self.output_buckets.add(created.len().into());
    }

    fn end_milestone(&mut self, _ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        Some(*self)
    }
}
