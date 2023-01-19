// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, ops::RangeBounds};

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};

use super::{BlockData, InputSource, MilestoneData, UnspentOutputData};
use crate::{
    db::{
        collections::{BlockCollection, MilestoneCollection, OutputCollection, ProtocolUpdateCollection},
        MongoDb,
    },
    tangle::ledger_updates::LedgerUpdateStore,
    types::tangle::MilestoneIndex,
};

#[async_trait]
impl InputSource for MongoDb {
    type Error = mongodb::error::Error;

    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
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

    async fn unspent_outputs(&self) -> Result<BoxStream<Result<UnspentOutputData, Self::Error>>, Self::Error> {
        todo!()
    }

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        let outputs = self
            .collection::<OutputCollection>()
            .get_ledger_updates(index)
            .await?
            .into_iter()
            .collect::<HashMap<_, _>>();
        Ok(LedgerUpdateStore { outputs })
    }
}
