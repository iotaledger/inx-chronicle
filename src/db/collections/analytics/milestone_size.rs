// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[derive(Clone, Debug, Default)]
#[allow(missing_docs)]
pub struct MilestoneSizeAnalyticsResult {
    pub total_milestone_payload_bytes: u64,
    pub total_tagged_data_payload_bytes: u64,
    pub total_transaction_payload_bytes: u64,
    pub total_treasury_transaction_payload_bytes: u64,
    pub total_milestone_bytes: u64,
}
