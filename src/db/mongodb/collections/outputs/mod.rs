// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod indexer;

use std::borrow::Borrow;

use futures::{Stream, StreamExt, TryStreamExt};
use iota_sdk::types::{
    block::{
        address::Address,
        output::{dto::OutputDto, AccountId, Output, OutputId},
        payload::signed_transaction::TransactionId,
        slot::{SlotCommitmentId, SlotIndex},
        BlockId,
    },
    TryFromDto,
};
use mongodb::{
    bson::{doc, to_bson, to_document},
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub use self::indexer::{
    AccountOutputsQuery, AnchorOutputsQuery, BasicOutputsQuery, DelegationOutputsQuery, FoundryOutputsQuery, IndexedId,
    NftOutputsQuery, OutputsResult,
};
use super::ledger_update::{LedgerOutputRecord, LedgerSpentRecord};
use crate::{
    db::{
        mongodb::{
            collections::ProtocolUpdateCollection, DbError, InsertIgnoreDuplicatesExt, MongoDbCollection,
            MongoDbCollectionExt,
        },
        MongoDb,
    },
    inx::ledger::{LedgerOutput, LedgerSpent},
    model::{address::AddressDto, native_token::NativeTokenDto, raw::Raw, tag::Tag, SerializeToBson},
};

/// Chronicle Output record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputDocument {
    #[serde(rename = "_id")]
    output_id: OutputId,
    output: Raw<Output>,
    metadata: OutputMetadata,
    details: OutputDetails,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Metadata for an output.
pub struct OutputMetadata {
    /// The ID of the block in which the output was included.
    pub block_id: BlockId,
    /// The slot in which the output was booked (created).
    pub slot_booked: SlotIndex,
    /// Commitment ID that includes the output.
    pub included_commitment_id: SlotCommitmentId,
    /// Optional spent metadata.
    pub spent_metadata: Option<SpentMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Metadata for a spent (consumed) output.
pub struct SpentMetadata {
    // Slot where the output was spent.
    pub slot_spent: SlotIndex,
    // Commitment ID that includes the spent output.
    pub commitment_id_spent: SlotCommitmentId,
    // Transaction ID that spent the output.
    pub transaction_id_spent: TransactionId,
}

/// The iota outputs collection.
pub struct OutputCollection {
    db: mongodb::Database,
    collection: mongodb::Collection<OutputDocument>,
    protocol_updates: ProtocolUpdateCollection,
}

#[async_trait::async_trait]
impl MongoDbCollection for OutputCollection {
    const NAME: &'static str = "iota_outputs";
    type Document = OutputDocument;

    fn instantiate(db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self {
            db: db.db(),
            collection,
            protocol_updates: db.collection(),
        }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), DbError> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "metadata.block_id": 1 })
                .options(
                    IndexOptions::builder()
                        .unique(false)
                        .name("metadata_block_id".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_indexer_indexes().await?;

        Ok(())
    }
}

/// Precalculated info and other output details.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct OutputDetails {
    kind: String,
    is_trivial_unlock: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    indexed_id: Option<IndexedId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<AddressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    governor_address: Option<AddressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    state_controller_address: Option<AddressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    storage_deposit_return_address: Option<AddressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timelock: Option<SlotIndex>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expiration: Option<SlotIndex>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expiration_return_address: Option<AddressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issuer: Option<AddressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sender: Option<AddressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tag: Option<Tag>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    block_issuer_expiry: Option<SlotIndex>,
    // TODO: staking feature
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    native_tokens: Vec<NativeTokenDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    validator: Option<AccountId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    account_address: Option<AccountId>,
}

impl From<&LedgerOutput> for OutputDocument {
    fn from(rec: &LedgerOutput) -> Self {
        Self {
            output_id: rec.output_id,
            output: rec.output.clone().into(),
            metadata: OutputMetadata {
                block_id: rec.block_id,
                slot_booked: rec.slot_booked,
                included_commitment_id: rec.commitment_id_included,
                spent_metadata: None,
            },
            details: OutputDetails {
                kind: rec.kind().to_owned(),
                is_trivial_unlock: rec
                    .output()
                    .unlock_conditions()
                    .map(|uc| {
                        uc.storage_deposit_return().is_none() && uc.expiration().is_none() && uc.timelock().is_none()
                    })
                    .unwrap_or(true),
                indexed_id: match rec.output() {
                    Output::Account(output) => Some(output.account_id_non_null(&rec.output_id).into()),
                    Output::Anchor(output) => Some(output.anchor_id_non_null(&rec.output_id).into()),
                    Output::Nft(output) => Some(output.nft_id_non_null(&rec.output_id).into()),
                    Output::Delegation(output) => Some(output.delegation_id_non_null(&rec.output_id).into()),
                    Output::Foundry(output) => Some(output.id().into()),
                    _ => None,
                },
                address: rec
                    .output()
                    .unlock_conditions()
                    .and_then(|uc| uc.address())
                    .map(|uc| uc.address().into()),
                governor_address: rec
                    .output()
                    .unlock_conditions()
                    .and_then(|uc| uc.governor_address())
                    .map(|uc| uc.address().into()),
                state_controller_address: rec
                    .output()
                    .unlock_conditions()
                    .and_then(|uc| uc.state_controller_address())
                    .map(|uc| uc.address().into()),
                storage_deposit_return_address: rec
                    .output()
                    .unlock_conditions()
                    .and_then(|uc| uc.storage_deposit_return())
                    .map(|uc| uc.return_address().into()),
                timelock: rec
                    .output()
                    .unlock_conditions()
                    .and_then(|uc| uc.timelock())
                    .map(|uc| uc.slot_index()),
                expiration: rec
                    .output()
                    .unlock_conditions()
                    .and_then(|uc| uc.expiration())
                    .map(|uc| uc.slot_index()),
                expiration_return_address: rec
                    .output()
                    .unlock_conditions()
                    .and_then(|uc| uc.expiration())
                    .map(|uc| uc.return_address().into()),
                issuer: rec
                    .output()
                    .features()
                    .and_then(|uc| uc.issuer())
                    .map(|uc| uc.address().into()),
                sender: rec
                    .output()
                    .features()
                    .and_then(|uc| uc.sender())
                    .map(|uc| uc.address().into()),
                tag: rec
                    .output()
                    .features()
                    .and_then(|uc| uc.tag())
                    .map(|uc| uc.tag())
                    .map(Tag::from_bytes),
                block_issuer_expiry: rec
                    .output()
                    .features()
                    .and_then(|uc| uc.block_issuer())
                    .map(|uc| uc.expiry_slot()),
                native_tokens: rec
                    .output()
                    .native_tokens()
                    .into_iter()
                    .flat_map(|t| t.iter())
                    .map(Into::into)
                    .collect(),
                validator: rec
                    .output()
                    .as_delegation_opt()
                    .map(|o| *o.validator_address().account_id()),
                account_address: rec.output().as_foundry_opt().map(|o| *o.account_address().account_id()),
            },
        }
    }
}

impl From<&LedgerSpent> for OutputDocument {
    fn from(rec: &LedgerSpent) -> Self {
        let mut res = Self::from(&rec.output);
        res.metadata.spent_metadata.replace(SpentMetadata {
            slot_spent: rec.slot_spent,
            commitment_id_spent: rec.commitment_id_spent,
            transaction_id_spent: rec.transaction_id_spent,
        });
        res
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[allow(missing_docs)]
pub struct OutputMetadataResult {
    pub output_id: OutputId,
    pub metadata: OutputMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct OutputWithMetadataResult {
    pub output_id: OutputId,
    pub output: Output,
    pub metadata: OutputMetadata,
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct BalanceResult {
    pub total_balance: u64,
    pub sig_locked_balance: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[allow(missing_docs)]
pub struct UtxoChangesResult {
    pub created_outputs: Vec<OutputId>,
    pub consumed_outputs: Vec<OutputId>,
}

/// Implements the queries for the core API.
impl OutputCollection {
    /// Upserts [`Outputs`](crate::model::utxo::Output) with their
    /// [`OutputMetadata`](crate::model::metadata::OutputMetadata).
    #[instrument(skip_all, err, level = "trace")]
    pub async fn update_spent_outputs(&self, outputs: impl IntoIterator<Item = &LedgerSpent>) -> Result<(), DbError> {
        // TODO: Replace `db.run_command` once the `BulkWrite` API lands in the Rust driver.
        let update_docs = outputs
            .into_iter()
            .map(|output| {
                Ok(doc! {
                    "q": { "_id": output.output.output_id.to_bson() },
                    "u": to_document(&OutputDocument::from(output))?,
                    "upsert": true,
                })
            })
            .collect::<Result<Vec<_>, DbError>>()?;

        if !update_docs.is_empty() {
            let mut command = doc! {
                "update": Self::NAME,
                "updates": update_docs,
            };
            if let Some(write_concern) = self.db.write_concern() {
                command.insert("writeConcern", to_bson(write_concern)?);
            }
            let selection_criteria = self.db.selection_criteria().cloned();
            let _ = self.db.run_command(command, selection_criteria).await?;
        }

        Ok(())
    }

    /// Inserts [`Outputs`](crate::model::utxo::Output) with their
    /// [`OutputMetadata`](crate::model::metadata::OutputMetadata).
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_unspent_outputs<I, B>(&self, outputs: I) -> Result<(), DbError>
    where
        I: IntoIterator<Item = B>,
        I::IntoIter: Send + Sync,
        B: Borrow<LedgerOutput>,
    {
        self.insert_many_ignore_duplicates(
            outputs.into_iter().map(|d| OutputDocument::from(d.borrow())),
            InsertManyOptions::builder().ordered(false).build(),
        )
        .await?;

        Ok(())
    }

    /// Get an [`Output`] by [`OutputId`].
    pub async fn get_output(&self, output_id: &OutputId) -> Result<Option<Output>, DbError> {
        Ok(self
            .aggregate::<OutputDto>(
                [
                    doc! { "$match": { "_id": output_id.to_bson() } },
                    doc! { "$replaceWith": "$output" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|o| Output::try_from_dto(o))
            .transpose()?)
    }

    /// Get an [`Output`] with its [`OutputMetadata`] by [`OutputId`].
    pub async fn get_output_with_metadata(
        &self,
        output_id: &OutputId,
        slot_index: SlotIndex,
    ) -> Result<Option<OutputWithMetadataResult>, DbError> {
        #[derive(Deserialize)]
        struct OutputWithMetadataRes {
            #[serde(rename = "_id")]
            output_id: OutputId,
            output: OutputDto,
            metadata: OutputMetadata,
        }

        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "_id": output_id.to_bson(),
                        "metadata.slot_booked": { "$lte": slot_index.0 }
                    } },
                    doc! { "$project": {
                        "output": "$output",
                        "metadata": {
                            "output_id": "$_id",
                            "block_id": "$metadata.block_id",
                            "booked": "$metadata.booked",
                            "spent_metadata": "$metadata.spent_metadata",
                        },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(
                |OutputWithMetadataRes {
                     output_id,
                     output,
                     metadata,
                 }| {
                    Result::<_, DbError>::Ok(OutputWithMetadataResult {
                        output_id,
                        output: Output::try_from_dto(output)?,
                        metadata,
                    })
                },
            )
            .transpose()?)
    }

    /// Get an [`OutputMetadata`] by [`OutputId`].
    pub async fn get_output_metadata(
        &self,
        output_id: &OutputId,
        slot_index: SlotIndex,
    ) -> Result<Option<OutputMetadataResult>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "_id": output_id.to_bson(),
                        "metadata.slot_booked": { "$lte": slot_index.0 }
                    } },
                    doc! { "$project": {
                        "output_id": "$_id",
                        "metadata": 1,
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?)
    }

    /// Stream all [`LedgerOutput`]s that were unspent at a given ledger index.
    pub async fn get_unspent_output_stream(
        &self,
        slot_index: SlotIndex,
    ) -> Result<impl Stream<Item = Result<LedgerOutput, DbError>>, DbError> {
        Ok(self
            .aggregate::<LedgerOutputRecord>(
                [
                    doc! { "$match": {
                        "metadata.slot_booked" : { "$lte": slot_index.0 },
                        "metadata.spent_metadata.slot_spent": { "$not": { "$lte": slot_index.0 } }
                    } },
                    doc! { "$project": {
                        "output_id": "$_id",
                        "block_id": "$metadata.block_id",
                        "booked": "$metadata.booked",
                        "output": "$output",
                        "rent_structure": "$details.rent_structure",
                    } },
                ],
                None,
            )
            .await?
            .map_err(Into::into)
            .map_ok(Into::into))
    }

    /// Get all created [`LedgerOutput`]s for the given slot index.
    pub async fn get_created_outputs(
        &self,
        slot_index: SlotIndex,
    ) -> Result<impl Stream<Item = Result<LedgerOutput, DbError>>, DbError> {
        Ok(self
            .aggregate::<LedgerOutputRecord>(
                [
                    doc! { "$match": {
                        "metadata.slot_booked": { "$eq": slot_index.0 }
                    } },
                    doc! { "$project": {
                        "output_id": "$_id",
                        "block_id": "$metadata.block_id",
                        "booked": "$metadata.booked",
                        "output": "$output",
                        "rent_structure": "$details.rent_structure",
                    } },
                ],
                None,
            )
            .await?
            .map_err(Into::into)
            .map_ok(Into::into))
    }

    /// Get all consumed [`LedgerSpent`]s for the given slot index.
    pub async fn get_consumed_outputs(
        &self,
        slot_index: SlotIndex,
    ) -> Result<impl Stream<Item = Result<LedgerSpent, DbError>>, DbError> {
        Ok(self
            .aggregate::<LedgerSpentRecord>(
                [
                    doc! { "$match": {
                        "metadata.spent_metadata.slot_spent": { "$eq": slot_index.0 }
                    } },
                    doc! { "$project": {
                        "output": {
                            "output_id": "$_id",
                            "block_id": "$metadata.block_id",
                            "booked": "$metadata.booked",
                            "output": "$output",
                            "rent_structure": "$details.rent_structure",
                        },
                        "spent_metadata": "$metadata.spent_metadata",
                    } },
                ],
                None,
            )
            .await?
            .map_err(Into::into)
            .map_ok(Into::into))
    }

    /// Get all ledger updates (i.e. consumed [`Output`]s) for the given slot index.
    pub async fn get_ledger_update_stream(
        &self,
        slot_index: SlotIndex,
    ) -> Result<impl Stream<Item = Result<(OutputId, Output), DbError>>, DbError> {
        #[derive(Deserialize)]
        struct Res {
            output_id: OutputId,
            output: OutputDto,
        }
        Ok(self
            .aggregate::<Res>(
                [
                    doc! { "$match": {
                        "metadata.spent_metadata.slot_spent": { "$eq": slot_index.0 }
                    } },
                    doc! { "$project": {
                        "output_id": "$_id",
                        "output": "$output",
                    } },
                ],
                None,
            )
            .await?
            .then(|res| async move {
                let res = res?;
                Ok((res.output_id, Output::try_from_dto(res.output)?))
            }))
    }

    /// Gets the spending transaction metadata of an [`Output`] by [`OutputId`].
    pub async fn get_spending_transaction_metadata(
        &self,
        output_id: &OutputId,
    ) -> Result<Option<SpentMetadata>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "_id": output_id.to_bson(),
                        "metadata.spent_metadata": { "$ne": null }
                    } },
                    doc! { "$replaceWith": "$metadata.spent_metadata" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?)
    }

    /// Sums the amounts of all outputs owned by the given [`Address`].
    pub async fn get_address_balance(
        &self,
        address: Address,
        slot_index: SlotIndex,
    ) -> Result<Option<BalanceResult>, DbError> {
        #[derive(Deserialize)]
        struct BalanceRes {
            total_balance: String,
            sig_locked_balance: String,
        }

        Ok(self
                .aggregate::<BalanceRes>(
                    [
                        // Look at all (at slot index o'clock) unspent output documents for the given address.
                        doc! { "$match": {
                            "details.address": address.to_bson(),
                            "metadata.slot_booked": { "$lte": slot_index.0 },
                            "metadata.spent_metadata.slot_spent": { "$not": { "$lte": slot_index.0 } }
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
                        } },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .map(|res|
                    BalanceResult {
                        total_balance: res.total_balance.parse().unwrap(),
                        sig_locked_balance: res.sig_locked_balance.parse().unwrap(),
                    }
                ))
    }

    /// Returns the changes to the UTXO ledger (as consumed and created output ids) that were applied at the given
    /// `index`. It returns `None` if the provided `index` is out of bounds (beyond Chronicle's ledger index). If
    /// the associated slot did not perform any changes to the ledger, the returned `Vec`s will be empty.
    pub async fn get_utxo_changes(
        &self,
        slot_index: SlotIndex,
        ledger_index: SlotIndex,
    ) -> Result<Option<UtxoChangesResult>, DbError> {
        if slot_index > ledger_index {
            Ok(None)
        } else {
            Ok(Some(
                self.aggregate(
                    [
                        doc! { "$match":
                           { "$or": [
                               { "metadata.slot_booked": slot_index.0  },
                               { "metadata.spent_metadata.slot_spent": slot_index.0 },
                           ] }
                        },
                        doc! { "$facet": {
                            "created_outputs": [
                                { "$match": { "metadata.slot_booked": slot_index.0  } },
                                { "$replaceWith": "$_id" },
                            ],
                            "consumed_outputs": [
                                { "$match": { "metadata.spent_metadata.slot_spent": slot_index.0 } },
                                { "$replaceWith": "$_id" },
                            ],
                        } },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .unwrap_or_default(),
            ))
        }
    }

    /// Get the address activity in a date
    pub async fn get_address_activity_count_in_range(
        &self,
        start_date: time::Date,
        end_date: time::Date,
    ) -> Result<usize, DbError> {
        #[derive(Deserialize)]
        struct Res {
            count: usize,
        }

        // TODO: handle missing params
        let protocol_params = self
            .protocol_updates
            .get_latest_protocol_parameters()
            .await?
            .expect("missing protocol parameters")
            .parameters;

        let (start_slot, end_slot) = (
            protocol_params.slot_index(start_date.midnight().assume_utc().unix_timestamp() as _),
            protocol_params.slot_index(end_date.midnight().assume_utc().unix_timestamp() as _),
        );

        Ok(self
            .aggregate::<Res>(
                [
                    doc! { "$match": { "$or": [
                        { "metadata.slot_booked": {
                            "$gte": start_slot.0,
                            "$lt": end_slot.0
                        } },
                        { "metadata.spent_metadata.slot_spent": {
                            "$gte": start_slot.0,
                            "$lt": end_slot.0
                        } },
                    ] } },
                    doc! { "$group": {
                        "_id": "$details.address",
                    } },
                    doc! { "$group": {
                        "_id": null,
                        "count": { "$sum": 1 }
                    } },
                ],
                None,
            )
            .await?
            .map_ok(|r| r.count)
            .try_next()
            .await?
            .unwrap_or_default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RichestAddresses {
    pub top: Vec<AddressStat>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AddressStat {
    pub address: Address,
    pub balance: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenDistribution {
    pub distribution: Vec<DistributionStat>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Statistics for a particular logarithmic range of balances
pub struct DistributionStat {
    /// The logarithmic index the balances are contained between: \[10^index..10^(index+1)\]
    pub index: u32,
    /// The number of unique addresses in this range
    pub address_count: u64,
    /// The total balance of the addresses in this range
    pub total_balance: u64,
}

impl OutputCollection {
    /// Create richest address statistics.
    pub async fn get_richest_addresses(
        &self,
        ledger_index: SlotIndex,
        top: usize,
    ) -> Result<RichestAddresses, DbError> {
        let top = self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.slot_booked": { "$lte": ledger_index.0 },
                        "metadata.spent_metadata.slot_spent": { "$not": { "$lte": ledger_index.0 } }
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
            .try_collect()
            .await?;
        Ok(RichestAddresses { top })
    }

    /// Create token distribution statistics.
    pub async fn get_token_distribution(&self, ledger_index: SlotIndex) -> Result<TokenDistribution, DbError> {
        let distribution = self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.slot_booked": { "$lte": ledger_index.0 },
                        "metadata.spent_metadata.slot_spent": { "$not": { "$lte": ledger_index.0 } }
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
            .try_collect()
            .await?;
        Ok(TokenDistribution { distribution })
    }
}
