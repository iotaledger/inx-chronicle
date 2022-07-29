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
        stardust::{
            block::{Address, BlockId, Output, OutputId},
            milestone::MilestoneTimestamp,
        },
        tangle::{MilestoneIndex, ProtocolInfo, ProtocolParameters, RentStructure},
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

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct BalancesResult {
    pub total_balance: u64,
    pub sig_locked_balance: u64,
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
        let collection = self.0.collection::<OutputDocument>(OutputDocument::COLLECTION);

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
                                "details.address": { "$exists": true } ,
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
        self.0
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
            .0
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
                .0
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
                .0
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
            .0
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
    pub async fn sum_balances_owned_by_address(&self, address: Address) -> Result<Option<BalancesResult>, Error> {
        #[derive(Deserialize, Default)]
        struct Amount {
            amount: f64,
        }

        #[derive(Deserialize, Default)]
        struct Balances {
            total_balance: Amount,
            sig_locked_balance: Amount,
        }

        let ledger_index = self.get_ledger_index().await?;
        if let Some(ledger_index) = ledger_index {
            let balances = self
                .0
                .collection::<Balances>(OutputDocument::COLLECTION)
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
                        doc! { "$facet": {
                            // Sum all output amounts (total balance).
                            "total_balance": [
                                { "$group" : {
                                    "_id": "null",
                                    "amount": { "$sum": { "$toDouble": "$output.amount" } },
                                }},
                            ],
                            // Sum only trivially unlockable output amounts (signature locked balance).
                            "sig_locked_balance": [
                                { "$match": { "details.is_trivial_unlock": true } },
                                { "$group" : {
                                    "_id": "null",
                                    "amount": { "$sum": { "$toDouble": "$output.amount" } },
                                } },
                            ],
                        } },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .map(bson::from_document::<Balances>)
                .transpose()?
                .unwrap_or_default();

            Ok(Some(BalancesResult {
                total_balance: balances.total_balance.amount as u64,
                sig_locked_balance: balances.sig_locked_balance.amount as u64,
                ledger_index,
            }))
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
                    self.0
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
    /// Gathers transaction analytics.
    pub async fn get_transaction_analytics(
        &self,
        start_timestamp: Option<MilestoneTimestamp>,
        end_timestamp: Option<MilestoneTimestamp>,
    ) -> Result<OutputAnalyticsResult, Error> {
        Ok(self
            .0
            .collection::<OutputAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "$nor": [
                            { "metadata.booked.milestone_timestamp": { "$lt": start_timestamp } },
                            { "metadata.booked.milestone_timestamp": { "$gte": end_timestamp } },
                        ],
                    } },
                    // First group the outputs into transactions
                    doc! { "$group" : {
                        "_id": "$metadata.output_id.transaction_id",
                        "amount": { "$sum": { "$toDecimal": "$output.amount" } },
                    }},
                    // Then aggregate transaction analytics
                    doc! { "$group" : {
                        "_id": null,
                        "count": { "$sum": 1 },
                        "total_value": { "$sum": "$amount" },
                    }},
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

    /// Gathers basic output analytics.
    pub async fn get_basic_analytics(
        &self,
        start_timestamp: Option<MilestoneTimestamp>,
        end_timestamp: Option<MilestoneTimestamp>,
    ) -> Result<OutputAnalyticsResult, Error> {
        Ok(self
            .0
            .collection::<OutputAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "$nor": [
                            { "metadata.booked.milestone_timestamp": { "$lt": start_timestamp } },
                            { "metadata.booked.milestone_timestamp": { "$gte": end_timestamp } },
                        ],
                        "output.kind": "basic"
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "count": { "$sum": 1 },
                        "total_value": { "$sum": "$amount" },
                    }},
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

    /// Gathers alias output analytics.
    pub async fn get_alias_analytics(
        &self,
        start_timestamp: Option<MilestoneTimestamp>,
        end_timestamp: Option<MilestoneTimestamp>,
    ) -> Result<OutputAnalyticsResult, Error> {
        Ok(self
            .0
            .collection::<OutputAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "$nor": [
                            { "metadata.booked.milestone_timestamp": { "$lt": start_timestamp } },
                            { "metadata.booked.milestone_timestamp": { "$gte": end_timestamp } },
                        ],
                        "output.kind": "alias"
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "count": { "$sum": 1 },
                        "total_value": { "$sum": "$amount" },
                    }},
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

    /// Gathers nft output analytics.
    pub async fn get_nft_analytics(
        &self,
        start_timestamp: Option<MilestoneTimestamp>,
        end_timestamp: Option<MilestoneTimestamp>,
    ) -> Result<OutputAnalyticsResult, Error> {
        Ok(self
            .0
            .collection::<OutputAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "$nor": [
                            { "metadata.booked.milestone_timestamp": { "$lt": start_timestamp } },
                            { "metadata.booked.milestone_timestamp": { "$gte": end_timestamp } },
                        ],
                        "output.kind": "nft"
                    } },
                    // First group the nfts by their ids
                    doc! { "$group" : {
                        "_id": "$output.nft_id",
                        "amount": { "$sum": { "$toDecimal": "$output.amount" } },
                    }},
                    doc! { "$group" : {
                        "_id": null,
                        "count": { "$sum": 1 },
                        "total_value": { "$sum": "$amount" },
                    }},
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

    /// Gathers nft output analytics.
    pub async fn get_foundry_analytics(
        &self,
        start_timestamp: Option<MilestoneTimestamp>,
        end_timestamp: Option<MilestoneTimestamp>,
    ) -> Result<OutputAnalyticsResult, Error> {
        Ok(self
            .0
            .collection::<OutputAnalyticsResult>(OutputDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "$nor": [
                            { "metadata.booked.milestone_timestamp": { "$lt": start_timestamp } },
                            { "metadata.booked.milestone_timestamp": { "$gte": end_timestamp } },
                        ],
                        "output.kind": "foundry"
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "count": { "$sum": 1 },
                        "total_value": { "$sum": "$amount" },
                    }},
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
        start_timestamp: Option<MilestoneTimestamp>,
        end_timestamp: Option<MilestoneTimestamp>,
    ) -> Result<Option<StorageDepositAnalyticsResult>, Error> {
        let protocol_params = self.get_protocol_parameters().await?;
        let ledger_index = self.get_ledger_index().await?;
        if let (
            Some(ProtocolInfo {
                parameters: ProtocolParameters { rent_structure, .. },
                ..
            }),
            Some(ledger_index),
        ) = (protocol_params, ledger_index)
        {
            #[derive(Default, Deserialize)]
            struct StorageDepositAnalytics {
                output_count: u64,
                storage_deposit_return_count: u64,
                storage_deposit_return_total_value: String,
                total_key_bytes: String,
                total_data_bytes: String,
                total_byte_cost: String,
            }

            let res = self
                .0
                .collection::<StorageDepositAnalytics>(OutputDocument::COLLECTION)
                .aggregate(
                    vec![
                        doc! { "$match": {
                            "metadata.booked.milestone_index": { "$lte": ledger_index },
                            "$or": [
                                { "metadata.spent_metadata.spent": null },
                                { "metadata.spent_metadata.spent.milestone_index": { "$gt": ledger_index } },
                            ],
                            "$nor": [
                                { "metadata.booked.milestone_timestamp": { "$lt": start_timestamp } },
                                { "metadata.booked.milestone_timestamp": { "$gte": end_timestamp } },
                                { "metadata.spent_metadata.spent.milestone_timestamp": { "$lt": end_timestamp } },
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
    }
}

/// Address analytics result.

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
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
        start_timestamp: Option<MilestoneTimestamp>,
        end_timestamp: Option<MilestoneTimestamp>,
    ) -> Result<Option<AddressAnalyticsResult>, Error> {
        Ok(self
            .0
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
                                        { "metadata.booked.milestone_timestamp": { "$lt": start_timestamp } },
                                        { "metadata.booked.milestone_timestamp": { "$gte": end_timestamp } },
                                    ] },
                                    { "$nor": [
                                        { "metadata.spent_metadata.spent.milestone_timestamp": { "$lt": start_timestamp } },
                                        { "metadata.spent_metadata.spent.milestone_timestamp": { "$gte": end_timestamp } },
                                    ] },
                                ],
                            } },
                            { "$group" : { "_id": "$details.address" }},
                            { "$count": "addresses" },
                        ],
                        "receiving": [
                            { "$match": { 
                                "$nor": [
                                    { "metadata.booked.milestone_timestamp": { "$lt": start_timestamp } },
                                    { "metadata.booked.milestone_timestamp": { "$gte": end_timestamp } },
                                ],
                             } },
                            { "$group" : { "_id": "$details.address" }},
                            { "$count": "addresses" },
                        ],
                        "sending": [
                            { "$match": { 
                                "$nor": [
                                    { "metadata.spent_metadata.spent.milestone_timestamp": { "$lt": start_timestamp } },
                                    { "metadata.spent_metadata.spent.milestone_timestamp": { "$gte": end_timestamp } },
                                ],
                             } },
                            { "$group" : { "_id": "$details.address" }},
                            { "$count": "addresses" },
                        ],
                    } },
                    doc! { "$project": {
                        "total_active_addresses": { "$first": "$total.addresses" },
                        "receiving_addresses": { "$first": "$receiving.addresses" },
                        "sending_addresses": { "$first": "$sending.addresses" },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?)
    }
}
