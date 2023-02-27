// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use super::*;
use crate::{
    analytics::{AnalyticsInterval, IntervalAnalytics},
    db::{collections::OutputCollection, MongoDb},
    model::stardust::Address,
};

#[derive(Debug, Default)]
pub(crate) struct AddressActivityMeasurement {
    pub(crate) count: usize,
}

/// Computes the number of addresses that were active during a given time interval.
#[allow(missing_docs)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct AddressActivityAnalytics {
    addresses: HashSet<Address>,
}

#[async_trait::async_trait]
impl IntervalAnalytics for AddressActivityMeasurement {
    type Measurement = Self;

    async fn handle_date_range(
        &mut self,
        start_date: time::Date,
        interval: AnalyticsInterval,
        db: &MongoDb,
    ) -> eyre::Result<Self::Measurement> {
        let count = db
            .collection::<OutputCollection>()
            .get_address_activity_count_in_range(start_date, interval.end_date(&start_date))
            .await?;
        Ok(AddressActivityMeasurement { count })
    }
}

impl Analytics for AddressActivityAnalytics {
    type Measurement = AddressActivityMeasurement;

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

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        AddressActivityMeasurement {
            count: std::mem::take(self).addresses.len(),
        }
    }
}
