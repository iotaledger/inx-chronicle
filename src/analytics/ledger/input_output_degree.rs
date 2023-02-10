// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::{collections::HashMap, fmt::Display};

use super::*;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum TransactionBucket {
    Small,    // 0-2
    Medium,   // 3-6
    Large,    // 7-9
    Huge,     // 10-49
    Gigantic, // 50-256(?)
}

impl From<usize> for TransactionBucket {
    fn from(value: usize) -> Self {
        match value {
            ..=2 => TransactionBucket::Small,
            3..=6 => TransactionBucket::Medium,
            7..=9 => TransactionBucket::Large,
            10..=49 => TransactionBucket::Huge,
            _ => TransactionBucket::Gigantic,
        }
    }
}

impl Display for TransactionBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TransactionBucket::Small => "small",
                TransactionBucket::Medium => "medium",
                TransactionBucket::Large => "large",
                TransactionBucket::Huge => "huge",
                TransactionBucket::Gigantic => "gigantic",
            }
        )
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TransactionDistributionMeasurement {
    pub(crate) input_buckets: HashMap<TransactionBucket, usize>,
    pub(crate) output_buckets: HashMap<TransactionBucket, usize>,
}

impl Analytics for TransactionDistributionMeasurement {
    type Measurement = PerMilestone<TransactionDistributionMeasurement>;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {
        let buckets = HashMap::from([
            (TransactionBucket::Small, 0),
            (TransactionBucket::Medium, 0),
            (TransactionBucket::Large, 0),
            (TransactionBucket::Huge, 0),
            (TransactionBucket::Gigantic, 0),
        ]);
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
