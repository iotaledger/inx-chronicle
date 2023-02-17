// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, fmt::Display};

use once_cell::sync::OnceCell;

use super::*;

fn get_empty_buckets() -> HashMap<Bucket, usize> {
    static EMPTY_BUCKETS: OnceCell<HashMap<Bucket, usize>> = OnceCell::new();

    EMPTY_BUCKETS
        .get_or_init(|| {
            let mut buckets = HashMap::from([
                (Bucket::Small, 0),
                (Bucket::Medium, 0),
                (Bucket::Large, 0),
                (Bucket::Huge, 0),
            ]);
            for i in 1..=7 {
                buckets.insert(Bucket::Single(i), 0);
            }
            buckets
        })
        .clone()
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum Bucket {
    Single(usize), // 1,..,7
    Small,         // [8..16)
    Medium,        // [16..32)
    Large,         // [32..64)
    Huge,          // [64..128)
}

impl From<usize> for Bucket {
    fn from(value: usize) -> Self {
        match value {
            0 => unreachable!("invalid transaction"),
            1..=7 => Bucket::Single(value),
            8..=15 => Bucket::Small,
            16..=31 => Bucket::Medium,
            32..=63 => Bucket::Large,
            _ => Bucket::Huge,
        }
    }
}

impl Display for Bucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bucket::Single(v) => write!(f, "{}", v),
            Bucket::Small => write!(f, "small"),
            Bucket::Medium => write!(f, "medium"),
            Bucket::Large => write!(f, "large"),
            Bucket::Huge => write!(f, "huge"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TransactionSizeMeasurement {
    pub(crate) input_buckets: HashMap<Bucket, usize>,
    pub(crate) output_buckets: HashMap<Bucket, usize>,
}

impl Default for TransactionSizeMeasurement {
    fn default() -> Self {
        Self {
            input_buckets: get_empty_buckets(),
            output_buckets: get_empty_buckets(),
        }
    }
}

impl Analytics for TransactionSizeMeasurement {
    type Measurement = TransactionSizeMeasurement;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        *self.input_buckets.entry(consumed.len().into()).or_default() += 1;
        *self.output_buckets.entry(created.len().into()).or_default() += 1;
    }

    fn end_milestone(&mut self, _ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        Some(std::mem::take(self))
    }
}
