// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeBounds;

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use thiserror::Error;

use super::{BlockData, InputSource, MilestoneData};
use crate::{
    db::{
        collections::{
            BlockCollection, ConfigurationUpdateCollection, MilestoneCollection, OutputCollection,
            ProtocolUpdateCollection,
        },
        MongoDb,
    },
    model::stardust::payload::milestone::MilestoneIndex,
    tangle::ledger_updates::LedgerUpdateStore,
};

#[derive(Debug, Error)]
pub enum MongoDbInputSourceError {
    #[error("missing milestone {0}")]
    MissingMilestone(MilestoneIndex),
    #[error("missing node config for ledger index {0}")]
    MissingNodeConfig(MilestoneIndex),
    #[error("missing protocol params for ledger index {0}")]
    MissingProtocolParams(MilestoneIndex),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
}

#[async_trait]
impl InputSource for MongoDb {
    type Error = MongoDbInputSourceError;

    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        use std::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(&idx) => idx.0,
            Bound::Excluded(&idx) => idx.0 + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&idx) => idx.0,
            Bound::Excluded(&idx) => idx.0 - 1,
            Bound::Unbounded => u32::MAX,
        };
        Ok(Box::pin(futures::stream::iter(start..=end).then(
            move |index| async move {
                let ((milestone_id, at, payload), protocol_params, node_config) = tokio::try_join!(
                    async {
                        self.collection::<MilestoneCollection>()
                            .get_milestone(index.into())
                            .await?
                            .ok_or(MongoDbInputSourceError::MissingMilestone(index.into()))
                    },
                    async {
                        Ok(self
                            .collection::<ProtocolUpdateCollection>()
                            .get_protocol_parameters_for_ledger_index(index.into())
                            .await?
                            .ok_or(MongoDbInputSourceError::MissingProtocolParams(index.into()))?
                            .parameters)
                    },
                    async {
                        Ok(self
                            .collection::<ConfigurationUpdateCollection>()
                            .get_node_configuration_for_ledger_index(index.into())
                            .await?
                            .ok_or(MongoDbInputSourceError::MissingNodeConfig(index.into()))?
                            .config)
                    }
                )?;
                Ok(MilestoneData {
                    milestone_id,
                    at,
                    payload,
                    protocol_params,
                    node_config,
                })
            },
        )))
    }

    /// Retrieves a stream of blocks and their metadata in white-flag order given a milestone index.
    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error> {
        Ok(Box::pin(
            self.collection::<BlockCollection>()
                .get_referenced_blocks_in_white_flag_order_stream(index)
                .await?
                .map_err(|e| e.into())
                .map_ok(|(block_id, block, raw, metadata)| BlockData {
                    block_id,
                    block,
                    raw,
                    metadata,
                }),
        ))
    }

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        let consumed = self
            .collection::<OutputCollection>()
            .get_consumed_outputs(index)
            .await?
            .try_collect()
            .await?;

        let created = self
            .collection::<OutputCollection>()
            .get_created_outputs(index)
            .await?
            .try_collect()
            .await?;

        Ok(LedgerUpdateStore::init(consumed, created))
    }
}
