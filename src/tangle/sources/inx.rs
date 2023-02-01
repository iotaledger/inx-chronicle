// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeBounds;

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};

use super::{BlockData, InputSource, MilestoneData};
use crate::{
    inx::{Inx, InxError, MarkerMessage, MilestoneRangeRequest},
    tangle::ledger_updates::LedgerUpdateStore,
    types::{ledger::MilestoneIndexTimestamp, tangle::MilestoneIndex},
};

#[async_trait]
impl InputSource for Inx {
    type Error = InxError;

    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        let mut inx = self.clone();
        Ok(Box::pin(
            inx.listen_to_confirmed_milestones(MilestoneRangeRequest::from_range(range))
                .await?
                .and_then(move |msg| {
                    let mut inx = inx.clone();
                    async move {
                        let node_config = inx.read_node_configuration().await?.into();
                        let payload = if let iota_types::block::payload::Payload::Milestone(payload) =
                            msg.milestone.milestone.inner_unverified()?
                        {
                            payload.into()
                        } else {
                            unreachable!("Raw milestone data has to contain a milestone payload");
                        };
                        Ok(MilestoneData {
                            // TODO: What do we do here, enhance the error type?
                            milestone_id: msg.milestone.milestone_info.milestone_id.unwrap(),
                            at: MilestoneIndexTimestamp {
                                milestone_index: msg.milestone.milestone_info.milestone_index,
                                milestone_timestamp: msg.milestone.milestone_info.milestone_timestamp.into(),
                            },
                            payload,
                            protocol_params: msg.current_protocol_parameters.params.inner_unverified()?.into(),
                            node_config,
                        })
                    }
                }),
        ))
    }

    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error> {
        let mut inx = self.clone();
        Ok(Box::pin(inx.read_milestone_cone(index.0.into()).await?.and_then(
            |msg| async move {
                Ok(BlockData {
                    block_id: msg.metadata.block_id,
                    block: msg.block.clone().inner_unverified()?.into(),
                    raw: msg.block.data(),
                    metadata: msg.metadata.into(),
                })
            },
        )))
    }

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        let mut inx = self.clone();
        let mut stream = inx.listen_to_ledger_updates((index.0..=index.0).into()).await?;
        let MarkerMessage {
            consumed_count,
            created_count,
            ..
            // TODO: What do we do here?
        } = stream.try_next().await?.unwrap().begin().unwrap();

        let consumed = stream
            .by_ref()
            .take(consumed_count)
            .map_ok(|update| {
                // Unwrap: Safe based on our knowledge of the stream layout
                update.consumed().unwrap()
            })
            .try_collect()
            .await?;

        let created = stream
            .take(created_count)
            .map_ok(|update| {
                // Unwrap: Safe based on our knowledge of the stream layout
                update.created().unwrap()
            })
            .try_collect()
            .await?;

        Ok(LedgerUpdateStore::init(consumed, created))
    }
}
