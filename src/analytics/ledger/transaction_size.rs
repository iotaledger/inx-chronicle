// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::{collections::HashMap, fmt::Display};

use super::*;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum TransactionSizeBucket {
    Small,    // 0-2
    Medium,   // 3-6
    Large,    // 7-9
    Huge,     // 10-49
    Gigantic, // 50-128
}

impl From<usize> for TransactionSizeBucket {
    fn from(value: usize) -> Self {
        match value {
            ..=2 => TransactionSizeBucket::Small,
            3..=6 => TransactionSizeBucket::Medium,
            7..=9 => TransactionSizeBucket::Large,
            10..=49 => TransactionSizeBucket::Huge,
            _ => TransactionSizeBucket::Gigantic,
        }
    }
}

impl Display for TransactionSizeBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TransactionSizeBucket::Small => "small",
                TransactionSizeBucket::Medium => "medium",
                TransactionSizeBucket::Large => "large",
                TransactionSizeBucket::Huge => "huge",
                TransactionSizeBucket::Gigantic => "gigantic",
            }
        )
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
        let buckets = HashMap::from([
            (TransactionSizeBucket::Small, 0),
            (TransactionSizeBucket::Medium, 0),
            (TransactionSizeBucket::Large, 0),
            (TransactionSizeBucket::Huge, 0),
            (TransactionSizeBucket::Gigantic, 0),
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
