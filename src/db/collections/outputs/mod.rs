// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod indexer;

use std::borrow::Borrow;

use futures::TryStreamExt;
use mongodb::{
    bson::{doc, to_bson, to_document},
    error::Error,
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub use self::indexer::{
    AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, IndexedId, NftOutputsQuery, OutputsResult,
};
use crate::{
    db::{
        mongodb::{InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    types::{
        ledger::{
            LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, OutputMetadata, RentStructureBytes, SpentMetadata,
        },
        stardust::block::{
            output::{Output, OutputId},
            Address, BlockId,
        },
        tangle::MilestoneIndex,
    },
};

/// Chronicle Output record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputDocument {
    #[serde(rename = "_id")]
    output_id: OutputId,
    output: Output,
    metadata: OutputMetadata,
    details: OutputDetails,
}

/// The stardust outputs collection.
pub struct OutputCollection {
    db: mongodb::Database,
    collection: mongodb::Collection<OutputDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for OutputCollection {
    const NAME: &'static str = "stardust_outputs";
    type Document = OutputDocument;

    fn instantiate(db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self {
            db: db.db.clone(),
            collection,
        }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), Error> {
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
pub struct OutputDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
    pub is_trivial_unlock: bool,
    pub rent_structure: RentStructureBytes,
}

impl From<&LedgerOutput> for OutputDocument {
    fn from(rec: &LedgerOutput) -> Self {
        let address = rec.output.owning_address().copied();
        let is_trivial_unlock = rec.output.is_trivial_unlock();

        Self {
            output_id: rec.output_id,
            output: rec.output.clone(),
            metadata: OutputMetadata {
                block_id: rec.block_id,
                booked: rec.booked,
                spent_metadata: None,
            },
            details: OutputDetails {
                address,
                is_trivial_unlock,
                rent_structure: rec.rent_structure,
            },
        }
    }
}

impl From<&LedgerSpent> for OutputDocument {
    fn from(rec: &LedgerSpent) -> Self {
        let mut res = Self::from(&rec.output);
        res.metadata.spent_metadata.replace(rec.spent_metadata);
        res
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[allow(missing_docs)]
pub struct OutputMetadataResult {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub booked: MilestoneIndexTimestamp,
    pub spent_metadata: Option<SpentMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
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
}

#[derive(Clone, Debug, Default, Deserialize)]
#[allow(missing_docs)]
pub struct UtxoChangesResult {
    pub created_outputs: Vec<OutputId>,
    pub consumed_outputs: Vec<OutputId>,
}

/// Implements the queries for the core API.
impl OutputCollection {
    /// Upserts [`Outputs`](crate::types::stardust::block::Output) with their
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    #[instrument(skip_all, err, level = "trace")]
    pub async fn update_spent_outputs(&self, outputs: impl IntoIterator<Item = &LedgerSpent>) -> Result<(), Error> {
        // TODO: Replace `db.run_command` once the `BulkWrite` API lands in the Rust driver.
        let update_docs = outputs
            .into_iter()
            .map(|output| {
                Ok(doc! {
                    "q": { "_id": output.output.output_id },
                    "u": to_document(&OutputDocument::from(output))?,
                    "upsert": true,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        if !update_docs.is_empty() {
            let mut command = doc! {
                "update": Self::NAME,
                "updates": update_docs,
            };
            if let Some(ref write_concern) = self.db.write_concern() {
                command.insert("writeConcern", to_bson(write_concern)?);
            }
            let selection_criteria = self.db.selection_criteria().cloned();
            let _ = self.db.run_command(command, selection_criteria).await?;
        }

        Ok(())
    }

    /// Inserts [`Outputs`](crate::types::stardust::block::Output) with their
    /// [`OutputMetadata`](crate::types::ledger::OutputMetadata).
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_unspent_outputs<I, B>(&self, outputs: I) -> Result<(), Error>
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
    pub async fn get_output(&self, output_id: &OutputId) -> Result<Option<Output>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": { "_id": output_id } },
                doc! { "$replaceWith": "$output" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Get an [`Output`] with its [`OutputMetadata`] by [`OutputId`].
    pub async fn get_output_with_metadata(
        &self,
        output_id: &OutputId,
        ledger_index: MilestoneIndex,
    ) -> Result<Option<OutputWithMetadataResult>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": {
                    "_id": output_id,
                    "metadata.booked.milestone_index": { "$lte": ledger_index }
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
        .await
    }

    /// Get an [`OutputMetadata`] by [`OutputId`].
    pub async fn get_output_metadata(
        &self,
        output_id: &OutputId,
        ledger_index: MilestoneIndex,
    ) -> Result<Option<OutputMetadataResult>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": {
                    "_id": &output_id,
                    "metadata.booked.milestone_index": { "$lte": ledger_index }
                } },
                doc! { "$project": {
                    "output_id": "$_id",
                    "block_id": "$metadata.block_id",
                    "booked": "$metadata.booked",
                    "spent_metadata": "$metadata.spent_metadata",
                } },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Gets the spending transaction metadata of an [`Output`] by [`OutputId`].
    pub async fn get_spending_transaction_metadata(
        &self,
        output_id: &OutputId,
    ) -> Result<Option<SpentMetadata>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": {
                    "_id": &output_id,
                    "metadata.spent_metadata": { "$ne": null }
                } },
                doc! { "$replaceWith": "$metadata.spent_metadata" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Sums the amounts of all outputs owned by the given [`Address`](crate::types::stardust::block::Address).
    pub async fn get_address_balance(
        &self,
        address: Address,
        ledger_index: MilestoneIndex,
    ) -> Result<Option<BalanceResult>, Error> {
        self
            .aggregate(
                vec![
                    // Look at all (at ledger index o'clock) unspent output documents for the given address.
                    doc! { "$match": {
                        "details.address": &address,
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
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
            .await
    }

    /// Returns the changes to the UTXO ledger (as consumed and created output ids) that were applied at the given
    /// `index`. It returns `None` if the provided `index` is out of bounds (beyond Chronicle's ledger index). If
    /// the associated milestone did not perform any changes to the ledger, the returned `Vec`s will be empty.
    pub async fn get_utxo_changes(
        &self,
        index: MilestoneIndex,
        ledger_index: MilestoneIndex,
    ) -> Result<Option<UtxoChangesResult>, Error> {
        if index > ledger_index {
            Ok(None)
        } else {
            Ok(Some(
                self.aggregate(
                    vec![doc! { "$facet": {
                        "created_outputs": [
                            { "$match": { "metadata.booked.milestone_index": index  } },
                            { "$replaceWith": "$_id" },
                        ],
                        "consumed_outputs": [
                            { "$match": { "metadata.spent_metadata.spent.milestone_index": index } },
                            { "$replaceWith": "$_id" },
                        ],
                    } }],
                    None,
                )
                .await?
                .try_next()
                .await?
                .unwrap_or_default(),
            ))
        }
    }
}

#[cfg(feature = "analytics")]
mod analytics {
    use decimal::d128;

    use super::*;
    use crate::{
        db::{
            collections::analytics::{
                AddressAnalytics, BaseTokenActivityAnalytics, LedgerOutputAnalytics, LedgerSizeAnalytics,
                OutputActivityAnalytics, UnclaimedTokenAnalytics, UnlockConditionAnalytics,
            },
            mongodb::MongoDbCollectionExt,
        },
        types::{
            stardust::block::output::{AliasId, NftId},
            tangle::MilestoneIndex,
        },
    };
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct OutputAnalyticsResult {
        pub count: u64,
        pub total_value: String,
    }

    impl OutputCollection {
        /// Gets analytics about unlock conditions.
        #[tracing::instrument(skip(self), err, level = "trace")]
        pub async fn get_unlock_condition_analytics(
            &self,
            ledger_index: MilestoneIndex,
        ) -> Result<UnlockConditionAnalytics, Error> {
            #[derive(Default, Deserialize)]
            struct Res {
                count: u64,
                value: d128,
            }

            let query = |kind: &'static str| async move {
                Result::<Res, Error>::Ok(
                    self.aggregate(
                        vec![
                            doc! { "$match": {
                                format!("output.{kind}"): { "$exists": true },
                                "metadata.booked.milestone_index": { "$lte": ledger_index },
                                "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                            } },
                            doc! { "$group": {
                                "_id": null,
                                "count": { "$sum": 1 },
                                "value": { "$sum": { "$toDecimal": "$output.amount" } },
                            } },
                            doc! { "$project": {
                                "count": 1,
                                "value": { "$toString": "$value" },
                            } },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .unwrap_or_default(),
                )
            };

            let (timelock, expiration, sdruc) = tokio::try_join!(
                query("timelock_unlock_condition"),
                query("expiration_unlock_condition"),
                query("storage_deposit_return_unlock_condition"),
            )?;

            Ok(UnlockConditionAnalytics {
                timelock_count: timelock.count,
                timelock_value: timelock.value,
                expiration_count: expiration.count,
                expiration_value: expiration.value,
                storage_deposit_return_count: sdruc.count,
                storage_deposit_return_value: sdruc.value,
            })
        }
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
    pub balance: String,
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
    pub total_balance: String,
}

impl OutputCollection {
    /// Create richest address statistics.
    pub async fn get_richest_addresses(
        &self,
        ledger_index: MilestoneIndex,
        top: usize,
    ) -> Result<RichestAddresses, Error> {
        let top = self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
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
    pub async fn get_token_distribution(&self, ledger_index: MilestoneIndex) -> Result<TokenDistribution, Error> {
        let distribution = self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
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
