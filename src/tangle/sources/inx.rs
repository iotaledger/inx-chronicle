// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeBounds;

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use thiserror::Error;

use super::{BlockData, InputSource, MilestoneData};
use crate::{
    inx::{Inx, InxError, MarkerMessage, MilestoneRangeRequest},
    model::tangle::{MilestoneIndex, MilestoneIndexTimestamp},
    tangle::ledger_updates::LedgerUpdateStore,
};

#[derive(Debug, Error)]
pub enum InxInputSourceError {
    #[error(transparent)]
    Inx(#[from] InxError),
    #[error("missing marker message in ledger update stream")]
    MissingMarkerMessage,
    #[error("missing milestone id for milestone index `{0}`")]
    MissingMilestoneInfo(MilestoneIndex),
    #[error("unexpected message in ledger update stream")]
    UnexpectedMessage,
}

#[async_trait]
impl InputSource for Inx {
    type Error = InxInputSourceError;

    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        let mut inx = self.clone();
        Ok(Box::pin(
            inx.listen_to_confirmed_milestones(MilestoneRangeRequest::from_range(range))
                .await?
                .map_err(Self::Error::from)
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
                            milestone_id: msg.milestone.milestone_info.milestone_id.ok_or(
                                Self::Error::MissingMilestoneInfo(msg.milestone.milestone_info.milestone_index),
                            )?,
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
        Ok(Box::pin(
            inx.read_milestone_cone(index.0.into())
                .await?
                .map_err(Self::Error::from)
                .and_then(|msg| async move {
                    Ok(BlockData {
                        block_id: msg.metadata.block_id,
                        block: msg.block.clone().inner_unverified()?.into(),
                        raw: msg.block.data(),
                        metadata: msg.metadata.into(),
                    })
                }),
        ))
    }

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        let mut inx = self.clone();
        let mut stream = inx.listen_to_ledger_updates((index.0..=index.0).into()).await?;
        let MarkerMessage {
            consumed_count,
            created_count,
            ..
        } = stream
            .try_next()
            .await?
            .ok_or(Self::Error::MissingMarkerMessage)?
            .begin()
            .ok_or(Self::Error::UnexpectedMessage)?;

        let consumed = stream
            .by_ref()
            .take(consumed_count)
            .map(|update| update?.consumed().ok_or(Self::Error::UnexpectedMessage))
            .try_collect()
            .await?;

        let created = stream
            .take(created_count)
            .map(|update| update?.created().ok_or(Self::Error::UnexpectedMessage))
            .try_collect()
            .await?;

        Ok(LedgerUpdateStore::init(consumed, created))
    }
}
