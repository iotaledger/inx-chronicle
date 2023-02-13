// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::{collections::HashMap, fmt::Display};

use super::*;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum TransactionSizeBucket {
    Small(usize), // 0-10
    Medium,       // 10-49
    Large,        // 50-128
}

impl From<usize> for TransactionSizeBucket {
    fn from(value: usize) -> Self {
        match value {
            ..=9 => TransactionSizeBucket::Small(value),
            10..=49 => TransactionSizeBucket::Medium,
            _ => TransactionSizeBucket::Large,
        }
    }
}

impl Display for TransactionSizeBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionSizeBucket::Small(v) => write!(f, "{}", v),
            TransactionSizeBucket::Medium => write!(f, "medium"),
            TransactionSizeBucket::Large => write!(f, "large"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TransactionSizeMeasurement {
    pub(crate) input_buckets: HashMap<TransactionSizeBucket, usize>,
    pub(crate) output_buckets: HashMap<TransactionSizeBucket, usize>,
}

impl Analytics for TransactionSizeMeasurement {
    type Measurement = PerMilestone<TransactionSizeMeasurement>;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {
        let mut buckets = HashMap::from([(TransactionSizeBucket::Medium, 0), (TransactionSizeBucket::Large, 0)]);
        for i in 0..10 {
            buckets.insert(TransactionSizeBucket::Small(i), 0);
        }
        *self = Self {
            input_buckets: buckets.clone(),
            output_buckets: buckets,
        };
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
