// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::collections::HashMap;

use super::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct InputOutputDegreeAnalytics {
    input_degree_hist: HashMap<u8, usize>,
    output_degree_hist: HashMap<u8, usize>,
    num_transactions: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct InputOutputDegreeDistributionMeasurement {
    pub(crate) input_degree_dist: HashMap<u8, f32>,
    pub(crate) output_degree_dist: HashMap<u8, f32>,
}

impl Analytics for InputOutputDegreeAnalytics {
    type Measurement = PerMilestone<InputOutputDegreeDistributionMeasurement>;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        // TODO: confirm that maximum number of inputs and outputs is 256
        self.input_degree_hist
            .entry(consumed.len() as u8)
            .and_modify(|count| *count += 1)
            .or_insert(1);
        self.output_degree_hist
            .entry(created.len() as u8)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        self.num_transactions += 1;
    }

    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        let mut dist = InputOutputDegreeDistributionMeasurement::default();
        for (degree, count) in self.input_degree_hist.iter().map(|(k, v)| (*k, *v)) {
            dist.input_degree_dist
                .insert(degree, count as f32 / self.num_transactions as f32);
        }
        for (degree, count) in self.output_degree_hist.iter().map(|(k, v)| (*k, *v)) {
            dist.output_degree_dist
                .insert(degree, count as f32 / self.num_transactions as f32);
        }

        Some(PerMilestone {
            at: *ctx.at(),
            inner: dist,
        })
    }
}
