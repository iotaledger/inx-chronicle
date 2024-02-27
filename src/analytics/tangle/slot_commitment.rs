// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::analytics::{Analytics, AnalyticsContext};

/// Slot size statistics.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct SlotCommitmentMeasurement {
    pub(crate) reference_mana_cost: u64,
}

impl Analytics for SlotCommitmentMeasurement {
    type Measurement = Self;

    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement {
        SlotCommitmentMeasurement {
            reference_mana_cost: ctx.slot_commitment().reference_mana_cost(),
        }
    }
}
