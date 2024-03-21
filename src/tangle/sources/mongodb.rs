// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::ops::RangeBounds;

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use iota_sdk::types::block::{payload::signed_transaction::TransactionId, slot::SlotIndex};
use thiserror::Error;

use super::InputSource;
use crate::{
    db::{
        mongodb::{
            collections::{BlockCollection, CommittedSlotCollection, OutputCollection},
            DbError,
        },
        MongoDb,
    },
    model::{
        block_metadata::{BlockWithMetadata, TransactionMetadata},
        ledger::LedgerUpdateStore,
        slot::Commitment,
    },
};

#[derive(Debug, Error)]
pub enum MongoDbInputSourceError {
    #[error("missing commitment for slot index {0}")]
    MissingCommitment(SlotIndex),
    #[error("missing metadata for transaction {0}")]
    MissingTransactionMetadata(TransactionId),
    #[error(transparent)]
    MongoDb(#[from] DbError),
}

#[async_trait]
impl InputSource for MongoDb {
    type Error = MongoDbInputSourceError;

    async fn commitment_stream(
        &self,
        range: impl RangeBounds<SlotIndex> + Send,
    ) -> Result<BoxStream<Result<Commitment, Self::Error>>, Self::Error> {
        use std::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(&idx) => idx.0,
            Bound::Excluded(&idx) => idx.0.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&idx) => idx.0,
            Bound::Excluded(&idx) => idx.0.saturating_sub(1),
            Bound::Unbounded => u32::MAX,
        };
        Ok(Box::pin(futures::stream::iter(start..=end).then(
            move |index| async move {
                let doc = self
                    .collection::<CommittedSlotCollection>()
                    .get_commitment(index.into())
                    .await?
                    .ok_or_else(|| MongoDbInputSourceError::MissingCommitment(index.into()))?;
                Ok(Commitment {
                    commitment_id: doc.commitment_id,
                    commitment: doc.commitment,
                })
            },
        )))
    }

    async fn accepted_blocks(
        &self,
        index: SlotIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadata, Self::Error>>, Self::Error> {
        Ok(Box::pin(
            self.collection::<BlockCollection>()
                .get_blocks_by_slot(index)
                .await?
                .map_err(Into::into),
        ))
    }

    async fn transaction_metadata(&self, transaction_id: TransactionId) -> Result<TransactionMetadata, Self::Error> {
        self.collection::<BlockCollection>()
            .get_transaction_metadata(&transaction_id)
            .await?
            .ok_or(MongoDbInputSourceError::MissingTransactionMetadata(transaction_id))
    }

    async fn ledger_updates(&self, index: SlotIndex) -> Result<LedgerUpdateStore, Self::Error> {
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
