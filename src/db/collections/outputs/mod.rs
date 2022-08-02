// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod indexer;

use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

pub use self::indexer::{
    AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, IndexedId, NftOutputsQuery, OutputsResult,
};
use crate::{
    db::MongoDb,
    types::{
        ledger::{MilestoneIndexTimestamp, OutputMetadata, OutputWithMetadata, RentStructureBytes, SpentMetadata},
        stardust::block::{Address, BlockId, Output, OutputId},
        tangle::MilestoneIndex,
    },
};

/// Chronicle Output record.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct OutputDocument {
    output: Output,
    metadata: OutputMetadata,
    details: OutputDetails,
}

impl OutputDocument {
    /// The stardust outputs collection name.
    const COLLECTION: &'static str = "stardust_outputs";
}

/// Precalculated info and other output details.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct OutputDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<Address>,
    is_trivial_unlock: bool,
    rent_structure: RentStructureBytes,
}

impl From<OutputWithMetadata> for OutputDocument {
    fn from(rec: OutputWithMetadata) -> Self {
        let address = rec.output.owning_address().copied();
        let is_trivial_unlock = rec.output.is_trivial_unlock();
        let rent_structure = rec.output.rent_structure();

        Self {
            output: rec.output,
            metadata: rec.metadata,
            details: OutputDetails {
                address,
                is_trivial_unlock,
                rent_structure,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OutputMetadataResult {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub booked: MilestoneIndexTimestamp,
    pub spent_metadata: Option<SpentMetadata>,
    pub ledger_index: MilestoneIndex,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OutputWithMetadataResult {
    pub output: Output,
    pub metadata: OutputMetadataResult,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct BalanceResult {
    pub total_balance: String,
    pub sig_locked_balance: String,
    pub ledger_index: MilestoneIndex,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[allow(missing_docs)]
pub struct UtxoChangesResult {
    pub created_outputs: Vec<OutputId>,
    pub consumed_outputs: Vec<OutputId>,
}

/// Implements the queries for the core API.
impl MongoDb {
    /// Creates output indexes.
    pub async fn create_output_indexes(&self) -> Result<(), Error> {
        let collection = self.db.collection::<OutputDocument>(OutputDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "metadata.output_id": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .name("output_id_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "details.address": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(false)
                            .name("address_index".to_string())
                            .partial_filter_expression(doc! {
                                "details.address": { "$exists": true },
                            })
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        self.create_indexer_output_indexes().await?;

        Ok(())
    }

    /// Upserts an [`Output`](crate::types::stardust::block::Output) together with its associated
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    pub async fn insert_output(&self, output: OutputWithMetadata) -> Result<(), Error> {
        self.db
            .collection::<OutputDocument>(OutputDocument::COLLECTION)
            .update_one(
                doc! { "metadata.output_id": output.metadata.output_id },
                doc! { "$set": bson::to_document(&OutputDocument::from(output))? },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    /// Get an [`Output`] by [`OutputId`].
    pub async fn get_output(&self, output_id: &OutputId) -> Result<Option<Output>, Error> {
        let output = self
            .db
            .collection::<Output>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "metadata.output_id": output_id } },
                    doc! { "$replaceWith": "$output" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(output)
    }

    /// Get an [`OutputWithMetadata`] by [`OutputId`].
    pub async fn get_output_with_metadata(
        &self,
        output_id: &OutputId,
    ) -> Result<Option<OutputWithMetadataResult>, Error> {
        let ledger_index = self.get_ledger_index().await?;
        if let Some(ledger_index) = ledger_index {
            let output = self
                .db
                .collection::<OutputWithMetadataResult>(OutputDocument::COLLECTION)
                .aggregate(
                    vec![
                        doc! { "$match": {
                            "metadata.output_id": &output_id,
                            "metadata.booked.milestone_index": { "$lte": ledger_index }
                        } },
                        doc! { "$set": {
                            // The max fn will not consider the spent milestone index if it is null,
                            // thus always setting the ledger index to our provided value
                            "metadata.ledger_index": { "$max": [ ledger_index, "$metadata.spent_metadata.spent.milestone_index" ] },
                        } },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .map(bson::from_document)
                .transpose()?;

            Ok(output)
        } else {
            Ok(None)
        }
    }

    /// Get an [`OutputMetadata`] by [`OutputId`].
    pub async fn get_output_metadata(&self, output_id: &OutputId) -> Result<Option<OutputMetadataResult>, Error> {
        let ledger_index = self.get_ledger_index().await?;
        if let Some(ledger_index) = ledger_index {
            let metadata = self
                .db
                .collection::<OutputMetadataResult>(OutputDocument::COLLECTION)
                .aggregate(
                    vec![
                        doc! { "$match": {
                            "metadata.output_id": &output_id,
                            "metadata.booked.milestone_index": { "$lte": ledger_index }
                        } },
                        doc! { "$set": {
                            // The max fn will not consider the spent milestone index if it is null,
                            // thus always setting the ledger index to our provided value
                            "metadata.ledger_index": { "$max": [ ledger_index, "$metadata.spent_metadata.spent.milestone_index" ] },
                        } },
                        doc! { "$replaceWith": "$metadata" },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .map(bson::from_document)
                .transpose()?;

            Ok(metadata)
        } else {
            Ok(None)
        }
    }

    /// Gets the spending transaction metadata of an [`Output`] by [`OutputId`].
    pub async fn get_spending_transaction_metadata(
        &self,
        output_id: &OutputId,
    ) -> Result<Option<SpentMetadata>, Error> {
        let metadata = self
            .db
            .collection::<SpentMetadata>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "metadata.output_id": &output_id } },
                    doc! { "$replaceWith": "$metadata.spent_metadata" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(metadata)
    }

    /// Sums the amounts of all outputs owned by the given [`Address`](crate::types::stardust::block::Address).
    pub async fn get_address_balance(&self, address: Address) -> Result<Option<BalanceResult>, Error> {
        let ledger_index = self.get_ledger_index().await?;
        if let Some(ledger_index) = ledger_index {
            let balances = self
                .db
                .collection::<BalanceResult>(OutputDocument::COLLECTION)
                .aggregate(
                    vec![
                        // Look at all (at ledger index o'clock) unspent output documents for the given address.
                        doc! { "$match": {
                            "details.address": &address,
                            "metadata.booked.milestone_index": { "$lte": ledger_index },
                            "$or": [
                                { "metadata.spent_metadata.spent": null },
                                { "metadata.spent_metadata.spent.milestone_index": { "$gt": ledger_index } },
                            ]
                        } },
                        doc! { "$group": {
                            "_id": null,
                            "total_balance": { "$sum": { "$toDecimal": "$output.amount" } },
                            "sig_locked_balance": { "$sum": { 
                                "$cond": [ { "$eq": [ "$details.is_trivial_unlock", true] }, { "$toDecimal": "$output.amount" }, 0 ]
                            } },
                        } },
                        doc! { "$project": {
                            "total_balance": { "$toString": "$total_balance" },
                            "sig_locked_balance": { "$toString": "$sig_locked_balance" },
                            "ledger_index": { "$literal": ledger_index },
                        } },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .map(bson::from_document::<BalanceResult>)
                .transpose()?;

            Ok(balances)
        } else {
            Ok(None)
        }
    }

    /// Returns the changes to the UTXO ledger (as consumed and created output ids) that were applied at the given
    /// `index`. It returns `None` if the provided `index` is out of bounds (beyond Chronicle's ledger index). If
    /// the associated milestone did not perform any changes to the ledger, the returned `Vec`s will be empty.
    pub async fn get_utxo_changes(&self, index: MilestoneIndex) -> Result<Option<UtxoChangesResult>, Error> {
        if let Some(ledger_index) = self.get_ledger_index().await? {
            if index > ledger_index {
                Ok(None)
            } else {
                Ok(Some(
                    self.db
                        .collection::<UtxoChangesResult>(OutputDocument::COLLECTION)
                        .aggregate(
                            vec![doc! { "$facet": {
                                "created_outputs": [
                                    { "$match": { "metadata.booked.milestone_index": index  } },
                                    { "$replaceWith": "$metadata.output_id" },
                                ],
                                "consumed_outputs": [
                                    { "$match": { "metadata.spent_metadata.spent.milestone_index": index } },
                                    { "$replaceWith": "$metadata.output_id" },
                                ],
                            } }],
                            None,
                        )
                        .await?
                        .try_next()
                        .await?
                        .map(bson::from_document::<UtxoChangesResult>)
                        .transpose()?
                        .unwrap_or_default(),
                ))
            }
        } else {
            Ok(None)
        }
    }
}
