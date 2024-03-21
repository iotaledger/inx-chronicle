// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::ops::RangeBounds;

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use iota_sdk::types::block::{payload::signed_transaction::TransactionId, slot::SlotIndex};
use thiserror::Error;

use super::InputSource;
use crate::{
    inx::{ledger::MarkerMessage, Inx, InxError, SlotRangeRequest},
    model::{
        block_metadata::{BlockWithMetadata, TransactionMetadata},
        ledger::LedgerUpdateStore,
        slot::Commitment,
    },
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

    async fn commitment_stream(
        &self,
        range: impl RangeBounds<SlotIndex> + Send,
    ) -> Result<BoxStream<Result<Commitment, Self::Error>>, Self::Error> {
        let mut inx = self.clone();
        Ok(Box::pin(
            inx.get_committed_slots(SlotRangeRequest::from_range(range))
                .await?
                .map_err(Self::Error::from),
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

    async fn transaction_metadata(&self, transaction_id: TransactionId) -> Result<TransactionMetadata, Self::Error> {
        let mut inx = self.clone();
        Ok(inx.get_transaction_metadata(transaction_id).await?)
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
