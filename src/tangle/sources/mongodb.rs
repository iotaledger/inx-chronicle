// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};

use super::{BlockData, InputSource, MilestoneData, MilestoneRange};
use crate::{
    db::{
        collections::{BlockCollection, MilestoneCollection, ProtocolUpdateCollection},
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
        range: MilestoneRange,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        // Need to have an owned value to hold in the iterator
        let db = self.clone();
        Ok(Box::pin(futures::stream::iter(*range.start..*range.end).then(
            move |index| {
                let db = db.clone();
                async move {
                    let (milestone_id, at, payload) = db
                        .collection::<MilestoneCollection>()
                        .get_milestone(index.into())
                        .await?
                        // TODO: what do we do with this?
                        .unwrap();
                    let protocol_params = db
                        .collection::<ProtocolUpdateCollection>()
                        .get_protocol_parameters_for_ledger_index(index.into())
                        .await?
                        // TODO: what do we do with this?
                        .unwrap()
                        .parameters;
                    Ok(MilestoneData {
                        milestone_id,
                        at,
                        payload,
                        protocol_params,
                    })
                }
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
