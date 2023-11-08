// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::ops::RangeBounds;

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use iota_sdk::types::block::slot::SlotIndex;
use thiserror::Error;

use super::{InputSource, SlotData};
use crate::{
    inx::{ledger::MarkerMessage, Inx, InxError, SlotRangeRequest},
    model::{block_metadata::BlockWithMetadata, ledger::LedgerUpdateStore},
};

#[derive(Debug, Error)]
pub enum InxInputSourceError {
    #[error(transparent)]
    Inx(#[from] InxError),
    #[error("missing marker message in ledger update stream")]
    MissingMarkerMessage,
    #[error("unexpected message in ledger update stream")]
    UnexpectedMessage,
}

#[async_trait]
impl InputSource for Inx {
    type Error = InxInputSourceError;

    async fn slot_stream(
        &self,
        range: impl RangeBounds<SlotIndex> + Send,
    ) -> Result<BoxStream<Result<SlotData, Self::Error>>, Self::Error> {
        let mut inx = self.clone();
        Ok(Box::pin(
            inx.get_committed_slots(SlotRangeRequest::from_range(range))
                .await?
                .map_err(Self::Error::from)
                .and_then(move |commitment| {
                    let mut inx = inx.clone();
                    async move {
                        let node_config = inx.get_node_configuration().await?.into();
                        Ok(SlotData {
                            commitment,
                            node_config,
                        })
                    }
                }),
        ))
    }

    async fn accepted_blocks(
        &self,
        index: SlotIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadata, Self::Error>>, Self::Error> {
        let mut inx = self.clone();
        Ok(Box::pin(
            inx.get_accepted_blocks_for_slot(index)
                .await?
                .map_err(Self::Error::from),
        ))
    }

    async fn ledger_updates(&self, index: SlotIndex) -> Result<LedgerUpdateStore, Self::Error> {
        let mut inx = self.clone();
        let mut stream = inx.get_ledger_updates((index.0..=index.0).into()).await?;
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
