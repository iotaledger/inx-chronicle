// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the tangle.

use serde::{Deserialize, Serialize};

pub(crate) use self::{
    block_activity::BlockActivityMeasurement, milestone_size::MilestoneSizeMeasurement,
    protocol_params::ProtocolParamsAnalytics,
};
use crate::{
    analytics::{Analytics, AnalyticsContext},
    tangle::BlockData,
    types::{stardust::block::Payload, tangle::ProtocolParameters},
};

mod block_activity;
mod milestone_size;
mod protocol_params;

#[cfg(test)]
mod test {
    use super::BlockActivityMeasurement;
    use crate::{
        analytics::{tangle::MilestoneSizeMeasurement, test::TestContext, Analytics},
        tangle::BlockData,
        types::{
            ledger::{BlockMetadata, ConflictReason, LedgerInclusionState},
            stardust::block::{Block, BlockId},
            tangle::MilestoneIndex,
        },
    };

    #[test]
    fn test_block_analytics() {
        let protocol_params = iota_types::block::protocol::protocol_parameters();

        let blocks = vec![
            Block::rand_treasury_transaction(&protocol_params),
            Block::rand_transaction(&protocol_params),
            Block::rand_milestone(&protocol_params),
            Block::rand_tagged_data(),
            Block::rand_no_payload(),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, block)| {
            let parents = block.parents.clone();
            BlockData {
                block_id: BlockId::rand(),
                block,
                raw: iota_types::block::rand::bytes::rand_bytes((i + 1) * 100),
                metadata: BlockMetadata {
                    parents,
                    is_solid: true,
                    should_promote: false,
                    should_reattach: false,
                    referenced_by_milestone_index: 1.into(),
                    milestone_index: 0.into(),
                    inclusion_state: match i {
                        0 => LedgerInclusionState::Included,
                        1 => LedgerInclusionState::Conflicting,
                        _ => LedgerInclusionState::NoTransaction,
                    },
                    conflict_reason: match i {
                        0 => ConflictReason::None,
                        1 => ConflictReason::InputUtxoNotFound,
                        _ => ConflictReason::None,
                    },
                    white_flag_index: i as u32,
                },
            }
        })
        .collect::<Vec<_>>();

        let mut block_activity = BlockActivityMeasurement::default();
        let mut milestone_size = MilestoneSizeMeasurement::default();

        let ctx = TestContext {
            at: MilestoneIndex(1).with_timestamp(12345.into()),
            params: protocol_params.into(),
        };

        for block_data in blocks.iter() {
            block_activity.handle_block(block_data, &ctx);
            milestone_size.handle_block(block_data, &ctx);
        }
        let block_activity_measurement = block_activity.take_measurement(&ctx);
        let milestone_size_measurement = milestone_size.take_measurement(&ctx);

        assert_eq!(block_activity_measurement.transaction_count, 1);
        assert_eq!(block_activity_measurement.treasury_transaction_count, 1);
        assert_eq!(block_activity_measurement.milestone_count, 1);
        assert_eq!(block_activity_measurement.tagged_data_count, 1);
        assert_eq!(block_activity_measurement.no_payload_count, 1);
        assert_eq!(block_activity_measurement.confirmed_count, 1);
        assert_eq!(block_activity_measurement.conflicting_count, 1);
        assert_eq!(block_activity_measurement.no_transaction_count, 3);

        assert_eq!(milestone_size_measurement.total_treasury_transaction_payload_bytes, 100);
        assert_eq!(milestone_size_measurement.total_transaction_payload_bytes, 200);
        assert_eq!(milestone_size_measurement.total_milestone_payload_bytes, 300);
        assert_eq!(milestone_size_measurement.total_tagged_data_payload_bytes, 400);
        assert_eq!(milestone_size_measurement.total_milestone_bytes, 1500);
    }
}
