// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::collections::HashSet;

use iota_sdk::types::block::{
    output::{AccountId, Output},
    payload::SignedTransactionPayload,
};
use serde::{Deserialize, Serialize};

use super::CountAndAmount;
use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
};

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct LedgerOutputMeasurement {
    pub(crate) account: AccountOutputMeasurement,
    pub(crate) basic: CountAndAmount,
    pub(crate) nft: CountAndAmount,
    pub(crate) foundry: CountAndAmount,
    pub(crate) anchor: CountAndAmount,
    pub(crate) delegation: CountAndAmount,
}

impl LedgerOutputMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            match output.output() {
                Output::Account(account_output) => {
                    measurement.account.count_and_amount.add_output(output);
                    if account_output.is_block_issuer() {
                        measurement.account.block_issuers_count += 1;
                    }
                }
                Output::Basic(_) => measurement.basic.add_output(output),
                Output::Nft(_) => measurement.nft.add_output(output),
                Output::Foundry(_) => measurement.foundry.add_output(output),
                Output::Anchor(_) => measurement.anchor.add_output(output),
                Output::Delegation(_) => measurement.delegation.add_output(output),
            }
        }
        measurement
    }

    fn wrapping_add(&mut self, rhs: Self) {
        self.account.wrapping_add(rhs.account);
        self.basic.wrapping_add(rhs.basic);
        self.nft.wrapping_add(rhs.nft);
        self.foundry.wrapping_add(rhs.foundry);
        self.anchor.wrapping_add(rhs.anchor);
        self.delegation.wrapping_add(rhs.delegation);
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        self.account.wrapping_sub(rhs.account);
        self.basic.wrapping_sub(rhs.basic);
        self.nft.wrapping_sub(rhs.nft);
        self.foundry.wrapping_sub(rhs.foundry);
        self.anchor.wrapping_sub(rhs.anchor);
        self.delegation.wrapping_sub(rhs.delegation);
    }
}

impl Analytics for LedgerOutputMeasurement {
    type Measurement = Self;

    fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        _ctx: &dyn AnalyticsContext,
    ) {
        fn map(ledger_output: &LedgerOutput) -> Option<AccountId> {
            ledger_output.output().as_account_opt().and_then(|output| {
                output
                    .is_block_issuer()
                    .then_some(output.account_id_non_null(&ledger_output.output_id))
            })
        }

        let issuer_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashSet<_>>();

        let issuer_outputs = created.iter().filter_map(map).collect::<HashSet<_>>();

        self.account.block_issuers_count = self
            .account
            .block_issuers_count
            .wrapping_add(issuer_outputs.difference(&issuer_inputs).count());
        self.account.block_issuers_count = self
            .account
            .block_issuers_count
            .wrapping_sub(issuer_inputs.difference(&issuer_outputs).count());

        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        self.wrapping_sub(consumed);
        self.wrapping_add(created);
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        *self
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct AccountOutputMeasurement {
    pub(crate) count_and_amount: CountAndAmount,
    pub(crate) block_issuers_count: usize,
}

impl AccountOutputMeasurement {
    fn wrapping_add(&mut self, rhs: Self) {
        self.count_and_amount.wrapping_add(rhs.count_and_amount);
        self.block_issuers_count = self.block_issuers_count.wrapping_add(rhs.block_issuers_count);
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        self.count_and_amount.wrapping_sub(rhs.count_and_amount);
        self.block_issuers_count = self.block_issuers_count.wrapping_sub(rhs.block_issuers_count);
    }
}
