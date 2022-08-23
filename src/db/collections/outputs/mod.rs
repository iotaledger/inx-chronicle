// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod indexer;

use futures::{StreamExt, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{IndexOptions, InsertManyOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tokio::try_join;
use tracing::instrument;

pub use self::indexer::{
    AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, IndexedId, NftOutputsQuery, OutputsResult,
};
use super::OutputKind;
use crate::{
    db::MongoDb,
    types::{
        ledger::{MilestoneIndexTimestamp, OutputMetadata, OutputWithMetadata, RentStructureBytes, SpentMetadata},
        stardust::block::{Address, BlockId, Output, OutputId},
        tangle::{MilestoneIndex, RentStructure},
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

    /// Inserts a batch of [`Output`](crate::types::stardust::block::Output)s together with their associated
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    #[instrument(name = "insert_outputs", skip_all, err, level = "trace")]
    pub async fn insert_outputs(&self, outputs: impl IntoIterator<Item = OutputWithMetadata>) -> Result<(), Error> {
        let docs = outputs.into_iter().map(OutputDocument::from);
        self.db
            .collection::<OutputDocument>(OutputDocument::COLLECTION)
            .insert_many(docs, InsertManyOptions::builder().ordered(false).build())
            .await?;
        Ok(())
    }

    /// Removes all [`OutputDocument`]s that are newer than a given [`MilestoneIndex`].
    #[instrument(name = "remove_outputs_newer_than_milestone", skip_all, err, level = "trace")]
    pub async fn remove_outputs_newer_than_milestone(&self, milestone_index: MilestoneIndex) -> Result<usize, Error> {
        let coll = &self.db.collection::<OutputDocument>(OutputDocument::COLLECTION);
        let (booked, spent) = try_join!(
            coll.delete_many(
                doc! {"metadata.booked.milestone_index": { "$gt": milestone_index } },
                None,
            ),
            coll.update_many(
                doc! {"metadata.spent.milestone_index": { "$gt": milestone_index } },
                doc! { "$unset": { "metadata.spent": "" } },
                None,
            )
        )?;

        Ok(booked.deleted_count as usize + spent.modified_count as usize)
    }

    /// Upserts an [`Output`](crate::types::stardust::block::Output) together with its associated
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    #[instrument(skip(self), err, level = "trace")]
    pub async fn upsert_output(&self, output: OutputWithMetadata) -> Result<(), Error> {
        if output.metadata.spent_metadata.is_none() {
            self.db
                .collection::<OutputDocument>(OutputDocument::COLLECTION)
                .update_one(
                    doc! { "metadata.output_id": output.metadata.output_id },
                    doc! { "$setOnInsert": bson::to_document(&OutputDocument::from(output))? },
                    UpdateOptions::builder().upsert(true).build(),
                )
                .await?;
        } else {
            self.db
                .collection::<OutputDocument>(OutputDocument::COLLECTION)
                .update_one(
                    doc! { "metadata.output_id": output.metadata.output_id },
                    doc! { "$set": bson::to_document(&OutputDocument::from(output))? },
                    UpdateOptions::builder().upsert(true).build(),
                )
                .await?;
        }

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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputAnalyticsResult {
    pub count: u64,
    pub total_value: String,
}

impl MongoDb {
    /// Gathers output analytics.
    pub async fn get_output_analytics<O: OutputKind>(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<OutputAnalyticsResult, Error> {
        let mut queries = vec![doc! {
            "$nor": [
                { "metadata.booked.milestone_index": { "$lt": start_index } },
                { "metadata.booked.milestone_index": { "$gte": end_index } },
            ]
        }];
        if let Some(kind) = O::kind() {
            queries.push(doc! { "output.kind": kind });
        }
        Ok(self
            .db
            .collection::<OutputAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "$and": queries } },
                    doc! { "$group" : {
                        "_id": null,
                        "count": { "$sum": 1 },
                        "total_value": { "$sum": { "$toDecimal": "$output.amount" } },
                    } },
                    doc! { "$project": {
                        "count": 1,
                        "total_value": { "$toString": "$total_value" },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?
            .unwrap_or_default())
    }

    /// Gathers unspent output analytics.
    pub async fn get_unspent_output_analytics<O: OutputKind>(
        &self,
        ledger_index: Option<MilestoneIndex>,
    ) -> Result<Option<OutputAnalyticsResult>, Error> {
        let ledger_index = match ledger_index {
            None => self.get_ledger_index().await?,
            i => i,
        };
        if let Some(ledger_index) = ledger_index {
            let mut queries = vec![doc! {
                "metadata.booked.milestone_index": { "$lte": ledger_index },
                "$or": [
                    { "metadata.spent_metadata.spent": null },
                    { "metadata.spent_metadata.spent.milestone_index": { "$gt": ledger_index } },
                ],
            }];
            if let Some(kind) = O::kind() {
                queries.push(doc! { "output.kind": kind });
            }
            Ok(Some(
                self.db
                    .collection::<OutputAnalyticsResult>(OutputDocument::COLLECTION)
                    .aggregate(
                        vec![
                            doc! { "$match": { "$and": queries } },
                            doc! { "$group" : {
                                "_id": null,
                                "count": { "$sum": 1 },
                                "total_value": { "$sum": { "$toDecimal": "$output.amount" } },
                            } },
                            doc! { "$project": {
                                "count": 1,
                                "total_value": { "$toString": "$total_value" },
                            } },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .map(bson::from_document)
                    .transpose()?
                    .unwrap_or_default(),
            ))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputDiffAnalyticsResult {
    pub created_count: u64,
    pub transferred_count: u64,
    pub burned_count: u64,
}

impl MongoDb {
    /// Gathers nft output analytics.
    pub async fn get_nft_output_analytics(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<OutputDiffAnalyticsResult, Error> {
        Ok(self
            .db
            .collection::<OutputDiffAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "output.kind": "nft",
                        "metadata.booked.milestone_index": { "$not": { "$gt": start_index } },
                    } },
                    doc! { "$facet": {
                        "start_state": [
                            { "$match": {
                                "$or": [
                                    { "metadata.spent_metadata.spent": null },
                                    { "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": start_index } } },
                                ],
                            } },
                            { "$project": {
                                "nft_id": "$output.nft_id"
                            } },
                        ],
                        "end_state": [
                            { "$match": {
                                "metadata.booked.milestone_index": { "$not": { "$lte": start_index } },
                                "$or": [
                                    { "metadata.spent_metadata.spent": null },
                                    { "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": end_index } } },
                                ],
                            } },
                            { "$project": {
                                "nft_id": "$output.nft_id"
                            } },
                        ],
                    } },
                    doc! { "$project": {
                        "created_count": { "$size": { "$setDifference": [ "$end_state", "$start_state" ] } },
                        "transferred_count": { "$size": { "$setIntersection": [ "$start_state", "$end_state" ] } },
                        "burned_count": { "$size": { "$setDifference": [ "$start_state", "$end_state" ] } },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?
            .unwrap_or_default())
    }

    /// Gathers foundry output analytics.
    pub async fn get_foundry_output_analytics(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<OutputDiffAnalyticsResult, Error> {
        Ok(self
            .db
            .collection::<OutputDiffAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "output.kind": "foundry",
                        "metadata.booked.milestone_index": { "$not": { "$gt": start_index } },
                    } },
                    doc! { "$facet": {
                        "start_state": [
                            { "$match": {
                                "$or": [
                                    { "metadata.spent_metadata.spent": null },
                                    { "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": start_index } } },
                                ],
                            } },
                            { "$project": {
                                "foundry_id": "$output.foundry_id"
                            } },
                        ],
                        "end_state": [
                            { "$match": {
                                "metadata.booked.milestone_index": { "$not": { "$lte": start_index } },
                                "$or": [
                                    { "metadata.spent_metadata.spent": null },
                                    { "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": end_index } } },
                                ],
                            } },
                            { "$project": {
                                "foundry_id": "$output.foundry_id"
                            } },
                        ],
                    } },
                    doc! { "$project": {
                        "created_count": { "$size": { "$setDifference": [ "$end_state", "$start_state" ] } },
                        "transferred_count": { "$size": { "$setIntersection": [ "$start_state", "$end_state" ] } },
                        "burned_count": { "$size": { "$setDifference": [ "$start_state", "$end_state" ] } },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?
            .unwrap_or_default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageDepositAnalyticsResult {
    pub output_count: u64,
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_total_value: String,
    pub total_key_bytes: String,
    pub total_data_bytes: String,
    pub total_byte_cost: String,
    pub ledger_index: MilestoneIndex,
    pub rent_structure: RentStructure,
}

impl MongoDb {
    /// Gathers byte cost and storage deposit analytics.
    pub async fn get_storage_deposit_analytics(
        &self,
        ledger_index: Option<MilestoneIndex>,
    ) -> Result<Option<StorageDepositAnalyticsResult>, Error> {
        let ledger_index = match ledger_index {
            None => self.get_ledger_index().await?,
            i => i,
        };
        if let Some(ledger_index) = ledger_index {
            if let Some(protocol_params) = self.get_protocol_parameters_for_ledger_index(ledger_index).await? {
                #[derive(Default, Deserialize)]
                struct StorageDepositAnalytics {
                    output_count: u64,
                    storage_deposit_return_count: u64,
                    storage_deposit_return_total_value: String,
                    total_key_bytes: String,
                    total_data_bytes: String,
                    total_byte_cost: String,
                }

                let rent_structure = protocol_params.parameters.rent_structure;

                let res = self
                    .db
                    .collection::<StorageDepositAnalytics>(OutputDocument::COLLECTION)
                    .aggregate(
                        vec![
                            doc! { "$match": {
                                "metadata.booked.milestone_index": { "$lte": ledger_index },
                                "$or": [
                                    { "metadata.spent_metadata.spent": null },
                                    { "metadata.spent_metadata.spent.milestone_index": { "$gt": ledger_index } },
                                ],
                            } },
                            doc! {
                                "$facet": {
                                    "all": [
                                        { "$group" : {
                                            "_id": null,
                                            "output_count": { "$sum": 1 },
                                            "total_key_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_key_bytes" } },
                                            "total_data_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_data_bytes" } },
                                        } },
                                    ],
                                    "storage_deposit": [
                                        { "$match": { "output.storage_deposit_return_unlock_condition": { "$exists": true } } },
                                        { "$group" : {
                                            "_id": null,
                                            "return_count": { "$sum": 1 },
                                            "return_total_value": { "$sum": { "$toDecimal": "$output.storage_deposit_return_unlock_condition.amount" } },
                                        } },
                                    ],
                                }
                            },
                            doc! {
                                "$set": {
                                    "total_byte_cost": { "$toString":
                                        { "$multiply": [
                                            rent_structure.v_byte_cost,
                                            { "$add": [
                                                { "$multiply": [ { "$first": "$all.total_key_bytes" }, rent_structure.v_byte_factor_key as i32 ] },
                                                { "$multiply": [ { "$first": "$all.total_data_bytes" }, rent_structure.v_byte_factor_data as i32 ] },
                                            ] },
                                        ] }
                                    }
                                }
                            },
                            doc! { "$project": {
                                "output_count": { "$first": "$all.output_count" },
                                "storage_deposit_return_count": { "$first": "$storage_deposit.return_count" },
                                "storage_deposit_return_total_value": { 
                                    "$toString": { "$first": "$storage_deposit.return_total_value" } 
                                },
                                "total_key_bytes": { 
                                    "$toString": { "$first": "$all.total_key_bytes" } 
                                },
                                "total_data_bytes": { 
                                    "$toString": { "$first": "$all.total_data_bytes" } 
                                },
                                "total_byte_cost": 1,
                            } },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .map(bson::from_document::<StorageDepositAnalytics>)
                    .transpose()?
                    .unwrap_or_default();

                Ok(Some(StorageDepositAnalyticsResult {
                    output_count: res.output_count,
                    storage_deposit_return_count: res.storage_deposit_return_count,
                    storage_deposit_return_total_value: res.storage_deposit_return_total_value,
                    total_key_bytes: res.total_key_bytes,
                    total_data_bytes: res.total_data_bytes,
                    total_byte_cost: res.total_byte_cost,
                    ledger_index,
                    rent_structure,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

/// Address analytics result.

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct AddressAnalyticsResult {
    /// The number of addresses used in the time period.
    pub total_active_addresses: u64,
    /// The number of addresses that received tokens in the time period.
    pub receiving_addresses: u64,
    /// The number of addresses that sent tokens in the time period.
    pub sending_addresses: u64,
}

impl MongoDb {
    /// Create aggregate statistics of all addresses.
    pub async fn get_address_analytics(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<AddressAnalyticsResult, Error> {
        Ok(self
            .db
            .collection::<AddressAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "details.address": { "$exists": true }
                    } },
                    doc! { "$facet": {
                        "total": [
                            { "$match": {
                                "$or": [
                                    { "$nor": [
                                        { "metadata.booked.milestone_index": { "$lt": start_index } },
                                        { "metadata.booked.milestone_index": { "$gte": end_index } },
                                    ] },
                                    { "$and": [
                                        { "metadata.spent_metadata.spent": { "exists": true } },
                                        { "$nor": [
                                            { "metadata.spent_metadata.spent.milestone_index": { "$lt": start_index } },
                                            { "metadata.spent_metadata.spent.milestone_index": { "$gte": end_index } },
                                        ] },
                                    ] },
                                ],
                            } },
                            { "$group" : { "_id": "$details.address" }},
                            { "$count": "addresses" },
                        ],
                        "receiving": [
                            { "$match": {
                                "$nor": [
                                    { "metadata.booked.milestone_index": { "$lt": start_index } },
                                    { "metadata.booked.milestone_index": { "$gte": end_index } },
                                ],
                             } },
                            { "$group" : { "_id": "$details.address" }},
                            { "$count": "addresses" },
                        ],
                        "sending": [
                            { "$match": {
                                "metadata.spent_metadata.spent": { "exists": true },
                                "$nor": [
                                    { "metadata.spent_metadata.spent.milestone_index": { "$lt": start_index } },
                                    { "metadata.spent_metadata.spent.milestone_index": { "$gte": end_index } },
                                ],
                             } },
                            { "$group" : { "_id": "$details.address" }},
                            { "$count": "addresses" },
                        ],
                    } },
                    doc! { "$project": {
                        "total_active_addresses": { "$max": [ 0, { "$first": "$total.addresses" } ] },
                        "receiving_addresses": { "$max": [ 0, { "$first": "$receiving.addresses" } ] },
                        "sending_addresses": { "$max": [ 0, { "$first": "$sending.addresses" } ] },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?
            .unwrap_or_default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RichestAddresses {
    pub top: Vec<AddressStat>,
    pub ledger_index: MilestoneIndex,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AddressStat {
    pub address: Address,
    pub balance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenDistribution {
    pub distribution: Vec<DistributionStat>,
    pub ledger_index: MilestoneIndex,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Statistics for a particular logarithmic range of balances
pub struct DistributionStat {
    /// The logarithmic index the balances are contained between: \[10^index..10^(index+1)\]
    pub index: u32,
    /// The number of unique addresses in this range
    pub address_count: u64,
    /// The total balance of the addresses in this range
    pub total_balance: String,
}

impl MongoDb {
    /// Create richest address statistics.
    pub async fn get_richest_addresses(
        &self,
        ledger_index: Option<MilestoneIndex>,
        top: usize,
    ) -> Result<Option<RichestAddresses>, Error> {
        let ledger_index = match ledger_index {
            None => self.get_ledger_index().await?,
            i => i,
        };
        if let Some(ledger_index) = ledger_index {
            let top = self
                .db
                .collection::<bson::Document>(OutputDocument::COLLECTION)
                .aggregate(
                    vec![
                        doc! { "$match": {
                            "details.address": { "$exists": true },
                            "metadata.booked.milestone_index": { "$lte": ledger_index },
                            "$or": [
                                { "metadata.spent_metadata.spent": null },
                                { "metadata.spent_metadata.spent.milestone_index": { "$gt": ledger_index } },
                            ]
                        } },
                        doc! { "$group" : {
                            "_id": "$details.address",
                            "balance": { "$sum": { "$toDecimal": "$output.amount" } },
                        } },
                        doc! { "$sort": { "balance": -1 } },
                        doc! { "$limit": top as i64 },
                        doc! { "$project": {
                            "_id": 0,
                            "address": "$_id",
                            "balance": { "$toString": "$balance" },
                        } },
                    ],
                    None,
                )
                .await?
                .map(|doc| Result::<_, Error>::Ok(bson::from_document(doc?)?))
                .try_collect()
                .await?;
            Ok(Some(RichestAddresses { top, ledger_index }))
        } else {
            Ok(Default::default())
        }
    }

    /// Create token distribution statistics.
    pub async fn get_token_distribution(
        &self,
        ledger_index: Option<MilestoneIndex>,
    ) -> Result<Option<TokenDistribution>, Error> {
        let ledger_index = match ledger_index {
            None => self.get_ledger_index().await?,
            i => i,
        };
        if let Some(ledger_index) = ledger_index {
            let distribution = self
                .db
                .collection::<bson::Document>(OutputDocument::COLLECTION)
                .aggregate(
                    vec![
                        doc! { "$match": {
                            "details.address": { "$exists": true },
                            "metadata.booked.milestone_index": { "$lte": ledger_index },
                            "$or": [
                                { "metadata.spent_metadata.spent": null },
                                { "metadata.spent_metadata.spent.milestone_index": { "$gt": ledger_index } },
                            ]
                        } },
                        doc! { "$group" : {
                            "_id": "$details.address",
                            "balance": { "$sum": { "$toDecimal": "$output.amount" } },
                        } },
                        doc! { "$set": { "index": { "$toInt": { "$log10": "$balance" } } } },
                        doc! { "$group" : {
                            "_id": "$index",
                            "address_count": { "$sum": 1 },
                            "total_balance": { "$sum": "$balance" },
                        } },
                        doc! { "$sort": { "_id": 1 } },
                        doc! { "$project": {
                            "_id": 0,
                            "index": "$_id",
                            "address_count": 1,
                            "total_balance": { "$toString": "$total_balance" },
                        } },
                    ],
                    None,
                )
                .await?
                .map(|doc| Result::<_, Error>::Ok(bson::from_document(doc?)?))
                .try_collect()
                .await?;
            Ok(Some(TokenDistribution {
                distribution,
                ledger_index,
            }))
        } else {
            Ok(Default::default())
        }
    }
}
