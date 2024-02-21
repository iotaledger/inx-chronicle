// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use iota_sdk::types::block::output::AccountId;

use crate::analytics::{Analytics, AnalyticsContext};

#[derive(Debug, Default)]
pub(crate) struct BlockIssuerMeasurement {
    pub(crate) active_issuer_count: usize,
}

/// Computes the number of block issuers that were active during a given time interval.
#[allow(missing_docs)]
#[derive(Debug, Default)]
pub(crate) struct BlockIssuerAnalytics {
    issuer_accounts: HashSet<AccountId>,
}

impl Analytics for BlockIssuerAnalytics {
    type Measurement = BlockIssuerMeasurement;

    fn handle_block(
        &mut self,
        block: &iota_sdk::types::block::Block,
        _metadata: &crate::model::block_metadata::BlockMetadata,
        _ctx: &dyn AnalyticsContext,
    ) {
        self.issuer_accounts.insert(block.issuer_id());
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        BlockIssuerMeasurement {
            active_issuer_count: std::mem::take(&mut self.issuer_accounts).len(),
        }
    }
}
