// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeBounds;

use async_trait::async_trait;
use futures::{stream::BoxStream, TryStreamExt};

use super::{BlockData, InputSource, MilestoneData};
use crate::{
    db::{collections::BlockCollection, MongoDb},
    tangle::ledger_updates::LedgerUpdateStore,
    types::tangle::MilestoneIndex,
};

#[async_trait]
impl InputSource for MongoDb {
    type Error = mongodb::error::Error;

    async fn milestone_stream(
        &self,
        _range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        todo!()
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
                .map_ok(|(block_id, block, raw, metadata)| BlockData {
                    block_id,
                    block,
                    raw,
                    metadata,
                }),
        ))
    }

    async fn ledger_updates(&self, _index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        todo!()
    }
}
