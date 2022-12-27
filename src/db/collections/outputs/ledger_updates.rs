// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{stream, Stream, StreamExt, TryStreamExt};
use mongodb::{bson::doc, error::Error};
use serde::Deserialize;
use tracing::instrument;

use super::{OutputCollection, OutputDocument};
use crate::{
    db::MongoDbCollectionExt,
    inx::{LedgerUpdateMarker, LedgerUpdateMessage, UnspentOutputMessage},
    types::{
        ledger::{LedgerOutput, LedgerSpent, SpentMetadata},
        tangle::MilestoneIndex,
    },
};

#[derive(Clone, Debug, Default, Deserialize)]
struct ConsumedCreatedCounts {
    consumed_outputs: usize,
    created_outputs: usize,
}

impl OutputCollection {
    async fn get_consumed_created_counts(&self, index: MilestoneIndex) -> Result<ConsumedCreatedCounts, Error> {
        // TODO: Highly inefficient
        let res = self.get_utxo_changes(index, index).await?;
        Ok(ConsumedCreatedCounts {
            consumed_outputs: res.as_ref().map(|c| c.consumed_outputs.len()).unwrap_or_default(),
            created_outputs: res.as_ref().map(|c| c.created_outputs.len()).unwrap_or_default(),
        })
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn get_created_outputs(
        &self,
        index: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<LedgerOutput, Error>>, Error> {
        let outputs = self.find(doc! { "metadata.booked.milestone_index": index }, None).await;
        Ok(outputs?.map_ok(|output: OutputDocument| LedgerOutput {
            output_id: output.output_id,
            output: output.output,
            block_id: output.metadata.block_id,
            booked: output.metadata.booked,
            rent_structure: output.details.rent_structure,
        }))
    }

    #[instrument(skip_all, err, level = "trace")]
    async fn get_consumed_outputs(
        &self,
        index: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<LedgerSpent, Error>>, Error> {
        let outputs = self
            .find(doc! { "metadata.spent_metadata.spent.milestone_index": index }, None)
            .await;
        Ok(outputs?.map_ok(|output: OutputDocument| LedgerSpent {
            output: LedgerOutput {
                output_id: output.output_id,
                output: output.output,
                block_id: output.metadata.block_id,
                booked: output.metadata.booked,
                rent_structure: output.details.rent_structure,
            },
            spent_metadata: SpentMetadata {
                transaction_id: output.metadata.spent_metadata.unwrap().transaction_id,
                spent: output.metadata.spent_metadata.unwrap().spent,
            },
        }))
    }

    /// Retrieves the ledger updates for a `milestone_index` as a stream.
    pub async fn get_ledger_update_stream(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateMessage, Error>>, Error> {
        let counts = self.get_consumed_created_counts(milestone_index).await?;
        let marker = LedgerUpdateMarker {
            milestone_index,
            consumed_count: counts.consumed_outputs,
            created_count: counts.created_outputs,
        };

        // Set up the individual streams.
        let begin_marker = LedgerUpdateMessage::Begin(marker.clone());
        let consumed_stream = self
            .get_consumed_outputs(milestone_index)
            .await?
            .map_ok(LedgerUpdateMessage::Consumed);
        let created_stream = self
            .get_created_outputs(milestone_index)
            .await?
            .map_ok(LedgerUpdateMessage::Created);
        let end_marker = LedgerUpdateMessage::End(marker);

        Ok(stream::once(async { Ok(begin_marker) })
            .chain(consumed_stream)
            .chain(created_stream)
            .chain(stream::once(async { Ok(end_marker) })))
    }

    /// Streams all updates that are part of the ledger state at `milestone_index`.
    pub async fn get_ledger_state(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<UnspentOutputMessage, Error>>, Error> {
        let outputs = self
            .find(
                doc! { "$match": {
                    "metadata.booked.milestone_index": { "$lte": milestone_index },
                    "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": milestone_index } }
                } },
                None,
            )
            .await;
        Ok(outputs?.map_ok(move |output: OutputDocument| UnspentOutputMessage {
            ledger_index: milestone_index,
            output: LedgerOutput {
                output_id: output.output_id,
                output: output.output,
                block_id: output.metadata.block_id,
                booked: output.metadata.booked,
                rent_structure: output.details.rent_structure,
            },
        }))
    }
}
