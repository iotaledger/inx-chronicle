// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{
    payload::{signed_transaction::TransactionCapabilityFlag, SignedTransactionPayload},
    protocol::WorkScore,
    Block,
};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::{
        block_metadata::BlockMetadata,
        ledger::{LedgerOutput, LedgerSpent},
    },
};

/// The type of payloads that occured within a single slot.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct ManaActivityMeasurement {
    pub(crate) rewards_claimed: u64,
    pub(crate) mana_burned: u64,
    pub(crate) bic_burned: u64,
}

impl Analytics for ManaActivityMeasurement {
    type Measurement = Self;

    fn handle_transaction(
        &mut self,
        payload: &SignedTransactionPayload,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        ctx: &dyn AnalyticsContext,
    ) {
        if payload
            .transaction()
            .capabilities()
            .has_capability(TransactionCapabilityFlag::BurnMana)
        {
            // TODO: Add reward mana
            let input_mana = consumed
                .iter()
                .map(|o| {
                    // Unwrap: acceptable risk
                    o.output()
                        .available_mana(ctx.protocol_parameters(), o.output.slot_booked, ctx.slot_index())
                        .unwrap()
                })
                .sum::<u64>();
            let output_mana = created.iter().map(|o| o.output().mana()).sum::<u64>()
                + payload.transaction().allotments().iter().map(|a| a.mana()).sum::<u64>();
            if input_mana > output_mana {
                self.mana_burned += input_mana - output_mana;
            }
        }
    }

    fn handle_block(&mut self, block: &Block, _metadata: &BlockMetadata, ctx: &dyn AnalyticsContext) {
        let rmc = ctx.slot_commitment().reference_mana_cost();
        if let Some(body) = block.body().as_basic_opt() {
            self.bic_burned += body.work_score(ctx.protocol_parameters().work_score_parameters()) as u64 * rmc;
        }
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        std::mem::take(self)
    }
}
