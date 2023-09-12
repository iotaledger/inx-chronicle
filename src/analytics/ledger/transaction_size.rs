// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct TransactionSizeBuckets {
    /// 1,..,7
    single: [usize; 7],
    /// [8..16)  
    pub(crate) small: usize,
    /// [16..32)
    pub(crate) medium: usize,
    /// [32..64)
    pub(crate) large: usize,
    /// [64..128)
    pub(crate) huge: usize,
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

    /// Get the single bucket for the given value.
    ///
    /// NOTE: only values 1 to 7 are valid.
    #[cfg(test)]
    pub(crate) const fn single(&self, i: usize) -> usize {
        debug_assert!(i > 0 && i < 8);
        self.single[i - 1]
    }

    /// Gets an enumerated iterator over the single buckets.
    pub(crate) fn single_buckets(&self) -> impl Iterator<Item = (usize, usize)> {
        (1..8).zip(self.single)
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct TransactionSizeMeasurement {
    pub(crate) input_buckets: TransactionSizeBuckets,
    pub(crate) output_buckets: TransactionSizeBuckets,
}

impl Analytics for TransactionSizeMeasurement {
    type Measurement = TransactionSizeMeasurement;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        self.input_buckets.add(consumed.len());
        self.output_buckets.add(created.len());
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        std::mem::take(self)
    }
}
