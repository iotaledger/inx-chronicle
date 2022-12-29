// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::block::payload as iota;

use super::TangleAnalytics;
use crate::inx::BlockWithMetadataMessage;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockPayloadStatistic {
    pub transaction_count: u32,
    pub treasury_transaction_count: u32,
    pub milestone_count: u32,
    pub tagged_data_count: u32,
    pub no_payload_count: u32,
}

#[derive(Clone, Debug)]
pub struct BlockPayloadAnalytics(BlockPayloadStatistic);

impl TangleAnalytics for BlockPayloadAnalytics {
    type Measurement = BlockPayloadStatistic;

    fn begin(&mut self) {
        self.0 = Default::default()
    }

    fn handle_block(&mut self, msg: &BlockWithMetadataMessage) {
        // Panic: Acceptable risk
        let block = msg.block.clone().inner_unverified().unwrap();
        match block.payload() {
            Some(iota::Payload::Transaction(_)) => self.0.transaction_count += 1,
            Some(iota::Payload::Milestone(_)) => self.0.milestone_count += 1,
            Some(iota::Payload::TreasuryTransaction(_)) => self.0.treasury_transaction_count += 1,
            Some(iota::Payload::TaggedData(_)) => self.0.tagged_data_count += 1,
            None => self.0.no_payload_count += 1,
        }
    }

    fn flush(&mut self) -> Option<Self::Measurement> {
        Some(self.0.clone())
    }
}
