// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::{collections::HashMap, fmt::Display};

use super::*;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum SizeBucket {
    Single(usize), // 0,1,..7
    Small,         // [8..16)
    Medium,        // [16..32)
    Large,         // [32..64)
    Huge,          // [64..128)
}

impl From<usize> for SizeBucket {
    fn from(value: usize) -> Self {
        match value {
            ..=7 => SizeBucket::Single(value),
            8..=15 => SizeBucket::Small,
            16..=31 => SizeBucket::Medium,
            32..=63 => SizeBucket::Large,
            _ => SizeBucket::Huge,
        }
    }
}

impl Display for SizeBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SizeBucket::Single(v) => write!(f, "{}", v),
            SizeBucket::Small => write!(f, "small"),
            SizeBucket::Medium => write!(f, "medium"),
            SizeBucket::Large => write!(f, "large"),
            SizeBucket::Huge => write!(f, "huge"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TransactionSizeMeasurement {
    pub(crate) input_buckets: HashMap<SizeBucket, usize>,
    pub(crate) output_buckets: HashMap<SizeBucket, usize>,
}

impl Analytics for TransactionSizeMeasurement {
    type Measurement = PerMilestone<TransactionSizeMeasurement>;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {
        let mut buckets = HashMap::from([
            (SizeBucket::Small, 0),
            (SizeBucket::Medium, 0),
            (SizeBucket::Large, 0),
            (SizeBucket::Huge, 0),
        ]);
        for i in 0..8 {
            buckets.insert(SizeBucket::Single(i), 0);
        }
        self.input_buckets = buckets.clone();
        self.output_buckets = buckets;
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        *self.input_buckets.entry(consumed.len().into()).or_default() += 1;
        *self.output_buckets.entry(created.len().into()).or_default() += 1;
    }

    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        Some(PerMilestone {
            at: *ctx.at(),
            inner: self.clone(),
        })
    }
}
