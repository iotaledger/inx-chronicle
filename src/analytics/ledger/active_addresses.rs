// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use time::{Duration, OffsetDateTime};

use super::*;
use crate::types::stardust::block::Address;

pub(crate) struct AddressActivityMeasurement {
    pub(crate) count: usize,
}

/// Computes the number of addresses that were active during a given time interval.
#[allow(missing_docs)]
pub(crate) struct AddressActivityAnalytics {
    start_time: OffsetDateTime,
    interval: Duration,
    addresses: HashSet<Address>,
    // Unfortunately, I don't see another way of implementing it using our current trait design
    measurement: Option<AddressActivityMeasurement>,
}

impl AddressActivityAnalytics {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(
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
            measurement: None,
        }
    }
}

impl Analytics for AddressActivityAnalytics {
    type Measurement = TimeInterval<AddressActivityMeasurement>;

    fn begin_milestone(&mut self, ctx: &dyn AnalyticsContext) {
        let end = self.start_time + self.interval;
        // Panic: The milestone timestamp is guaranteed to be valid.
        if OffsetDateTime::try_from(ctx.at().milestone_timestamp).unwrap() > end {
            self.measurement = Some(AddressActivityMeasurement {
                count: self.addresses.len(),
            });
            self.addresses.clear();
            self.start_time = end;
        }
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        for output in consumed {
            if let Some(a) = output.owning_address() {
                self.addresses.insert(*a);
            }
        }

        for output in created {
            if let Some(a) = output.owning_address() {
                self.addresses.insert(*a);
            }
        }
    }

    fn end_milestone(&mut self, _ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        self.measurement.take().map(|m| TimeInterval {
            from: self.start_time,
            to_exclusive: self.start_time + self.interval,
            inner: m,
        })
    }
}
