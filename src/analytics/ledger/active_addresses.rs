// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use time::{Duration, OffsetDateTime};

use super::TransactionAnalytics;
use crate::{
    db::collections::analytics::DailyActiveAddressAnalyticsResult,
    types::{
        ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::Address,
    },
};

/// Computes the number of addresses that were active during a given time interval.
#[allow(missing_docs)]
pub struct AddressActivity {
    pub start_time: OffsetDateTime,
    pub interval: Duration,
    addresses: HashSet<Address>,
    // Unfortunately, I don't see another way of implementing it using our current trait design
    flush: Option<usize>,
}

impl AddressActivity {
    /// Initialize the analytics by reading the current ledger state.
    pub fn init<'a>(
        start_time: OffsetDateTime,
        interval: Duration,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
    ) -> Self {
        let addresses = unspent_outputs
            .into_iter()
            .filter_map(|output| {
                let booked = OffsetDateTime::try_from(output.booked.milestone_timestamp).unwrap();
                if (start_time <= booked) && (booked < start_time + interval) {
                    output.owning_address().cloned()
                } else {
                    None
                }
            })
            .collect();
        Self {
            start_time,
            interval,
            addresses,
            flush: None,
        }
    }
}

impl TransactionAnalytics for AddressActivity {
    type Measurement = DailyActiveAddressAnalyticsResult;

    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp) {
        let end = self.start_time + self.interval;
        // Panic: The milestone timestamp is guaranteed to be valid.
        if OffsetDateTime::try_from(at.milestone_timestamp).unwrap() > end {
            self.flush = Some(self.addresses.len());
            self.addresses.clear();
            self.start_time = end;
        }
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        for input in consumed {
            if let Some(a) = input.owning_address() {
                self.addresses.insert(*a);
            }
        }

        for output in created {
            if let Some(a) = output.owning_address() {
                self.addresses.insert(*a);
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        self.flush
            .take()
            .map(|count| DailyActiveAddressAnalyticsResult { count: count as _ })
    }
}
