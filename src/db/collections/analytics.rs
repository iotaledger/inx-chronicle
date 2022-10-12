// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use decimal::d128;
use futures::TryStreamExt;
use mongodb::{
    bson::doc,
    error::{Error, ErrorKind},
    options::{CreateCollectionOptions, TimeseriesOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};

use super::{outputs::OutputDocument, BlockCollection, OutputCollection, OutputKind};
use crate::{
    db::{MongoDb, MongoDbCollection, MongoDbCollectionExt},
    types::{
        stardust::{
            block::{
                output::{AliasOutput, BasicOutput, FoundryOutput, NftOutput},
                Output,
            },
            milestone::MilestoneTimestamp,
        },
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyticsDocument {
    #[serde(rename = "_id")]
    milestone_index: MilestoneIndex,
    milestone_timestamp: MilestoneTimestamp,
    analytics: Analytics,
}

/// Holds analytics about stardust data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Analytics {
    addresses: AddressAnalytics,
    outputs: HashMap<String, OutputAnalytics>,
    unspent_outputs: HashMap<String, OutputAnalytics>,
    storage_deposits: StorageDepositAnalytics,
    claimed_tokens: ClaimedTokensAnalytics,
    milestone_activity: MilestoneActivityAnalytics,
}

impl Analytics {
    /// Process a batch of outputs.
    pub fn analyze_batch<'a, I>(&mut self, outputs: I)
    where
        I: IntoIterator<Item = &'a OutputDocument>,
    {
        for output in outputs {
            if output.details.address.is_some() {
                self.addresses.total_active_addresses += 1;
                if output.metadata.spent_metadata.is_some() {
                    self.addresses.sending_addresses += 1;
                } else {
                    self.addresses.receiving_addresses += 1;
                }
            }
            let output_analytics = self.outputs.entry(output.output.kind().to_string()).or_default();
            output_analytics.count += 1;
            output_analytics.total_value += output.output.amount().0.into();

            let unspent_output_analytics = self.outputs.entry(output.output.kind().to_string()).or_default();
            if output.metadata.spent_metadata.is_some() {
                unspent_output_analytics.count -= 1;
                unspent_output_analytics.total_value -= output.output.amount().0.into();
            } else {
                unspent_output_analytics.count += 1;
                unspent_output_analytics.total_value += output.output.amount().0.into();
            }

            self.storage_deposits.output_count += 1;
            self.storage_deposits.total_data_bytes += output.details.rent_structure.num_data_bytes.into();
            self.storage_deposits.total_key_bytes += output.details.rent_structure.num_key_bytes.into();
            match output.output {
                Output::Basic(BasicOutput {
                    storage_deposit_return_unlock_condition: Some(uc),
                    ..
                })
                | Output::Nft(NftOutput {
                    storage_deposit_return_unlock_condition: Some(uc),
                    ..
                }) => {
                    self.storage_deposits.storage_deposit_return_count += 1;
                    self.storage_deposits.storage_deposit_return_total_value += uc.amount.0.into();
                }
                _ => (),
            }
            // TODO: Claimed tokens
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct AddressAnalytics {
    /// The number of addresses used in the time period.
    pub total_active_addresses: u64,
    /// The number of addresses that received tokens in the time period.
    pub receiving_addresses: u64,
    /// The number of addresses that sent tokens in the time period.
    pub sending_addresses: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputAnalytics {
    pub count: u64,
    pub total_value: d128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StorageDepositAnalytics {
    pub output_count: u64,
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_total_value: d128,
    pub total_key_bytes: d128,
    pub total_data_bytes: d128,
}

impl StorageDepositAnalytics {
    pub fn total_byte_cost(&self, protocol_params: &ProtocolParameters) -> d128 {
        let rent_structure = protocol_params.rent_structure;
        d128::from(rent_structure.v_byte_cost)
            * ((self.total_data_bytes * d128::from(rent_structure.v_byte_factor_data as u32))
                + (self.total_data_bytes * d128::from(rent_structure.v_byte_factor_data as u32)))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ClaimedTokensAnalytics {
    pub count: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MilestoneActivityAnalytics {
    /// The number of blocks referenced by a milestone.
    pub num_blocks: u32,
    /// The number of blocks referenced by a milestone that contain a payload.
    pub num_tx_payload: u32,
    /// The number of blocks containing a treasury transaction payload.
    pub num_treasury_tx_payload: u32,
    /// The number of blocks containing a milestone payload.
    pub num_milestone_payload: u32,
    /// The number of blocks containing a tagged data payload.
    pub num_tagged_data_payload: u32,
    /// The number of blocks referenced by a milestone that contain no payload.
    pub num_no_payload: u32,
    /// The number of blocks containing a confirmed transaction.
    pub num_confirmed_tx: u32,
    /// The number of blocks containing a conflicting transaction.
    pub num_conflicting_tx: u32,
    /// The number of blocks containing no transaction.
    pub num_no_tx: u32,
}

/// The time-series analytics collection.
pub struct AnalyticsCollection {
    collection: mongodb::Collection<AnalyticsDocument>,
    output_collection: OutputCollection,
    block_collection: BlockCollection,
}

#[async_trait::async_trait]
impl MongoDbCollection for AnalyticsCollection {
    const NAME: &'static str = "stardust_analytics";
    type Document = AnalyticsDocument;

    fn instantiate(db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self {
            collection,
            output_collection: db.collection(),
            block_collection: db.collection(),
        }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_collection(&self, db: &MongoDb) -> Result<(), Error> {
        if let Err(e) = db
            .db
            .create_collection(
                Self::NAME,
                CreateCollectionOptions::builder()
                    .timeseries(
                        TimeseriesOptions::builder()
                            .time_field("milestone_timestamp".to_string())
                            .meta_field(None)
                            .granularity(None)
                            .build(),
                    )
                    .build(),
            )
            .await
        {
            match &*e.kind {
                ErrorKind::Command(ce) => {
                    if ce.code_name == "NamespaceExists" {
                        return Ok(());
                    } else {
                        return Err(e);
                    }
                }
                _ => return Err(e),
            }
        }
        Ok(())
    }

    async fn create_indexes(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl AnalyticsCollection {
    /// Upserts an analytics record.
    pub async fn upsert_analytics(
        &self,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
        analytics: Analytics,
    ) -> Result<(), Error> {
        self.update_one(
            doc! { "_id": milestone_index },
            doc! {
                "$set": {
                    "analytics": mongodb::bson::to_document(&analytics)?
                },
                "$setOnInsert": {
                    "milestone_timestamp": milestone_timestamp,
                }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }

    /// Gets all analytics for a milestone index, fetching the data if it is not available in the analytics collection.
    pub async fn get_all_analytics(&self, milestone_index: MilestoneIndex) -> Result<Analytics, Error> {
        Ok(match self.find_one(doc! { "_id": milestone_index }, None).await? {
            Some(res) => res,
            None => {
                let addresses = self
                    .output_collection
                    .get_address_analytics(milestone_index, milestone_index + 1)
                    .await?;
                let mut outputs = HashMap::new();
                outputs.insert(
                    BasicOutput::kind().unwrap().to_string(),
                    self.output_collection
                        .get_output_analytics::<BasicOutput>(milestone_index, milestone_index + 1)
                        .await?,
                );
                outputs.insert(
                    AliasOutput::kind().unwrap().to_string(),
                    self.output_collection
                        .get_output_analytics::<AliasOutput>(milestone_index, milestone_index + 1)
                        .await?,
                );
                outputs.insert(
                    NftOutput::kind().unwrap().to_string(),
                    self.output_collection
                        .get_output_analytics::<NftOutput>(milestone_index, milestone_index + 1)
                        .await?,
                );
                outputs.insert(
                    FoundryOutput::kind().unwrap().to_string(),
                    self.output_collection
                        .get_output_analytics::<FoundryOutput>(milestone_index, milestone_index + 1)
                        .await?,
                );
                let mut unspent_outputs = HashMap::new();
                unspent_outputs.insert(
                    BasicOutput::kind().unwrap().to_string(),
                    self.output_collection
                        .get_unspent_output_analytics::<BasicOutput>(milestone_index)
                        .await?,
                );
                unspent_outputs.insert(
                    AliasOutput::kind().unwrap().to_string(),
                    self.output_collection
                        .get_unspent_output_analytics::<AliasOutput>(milestone_index)
                        .await?,
                );
                unspent_outputs.insert(
                    NftOutput::kind().unwrap().to_string(),
                    self.output_collection
                        .get_unspent_output_analytics::<NftOutput>(milestone_index)
                        .await?,
                );
                unspent_outputs.insert(
                    FoundryOutput::kind().unwrap().to_string(),
                    self.output_collection
                        .get_unspent_output_analytics::<FoundryOutput>(milestone_index)
                        .await?,
                );
                let storage_deposits = self
                    .output_collection
                    .get_storage_deposit_analytics(milestone_index)
                    .await?;
                let claimed_tokens = self
                    .output_collection
                    .get_claimed_token_analytics(milestone_index)
                    .await?;
                let milestone_activity = self.block_collection.get_milestone_activity(milestone_index).await?;
                Analytics {
                    addresses,
                    outputs,
                    unspent_outputs,
                    storage_deposits,
                    claimed_tokens,
                    milestone_activity,
                }
            }
        })
    }

    /// Create aggregate statistics of all addresses.
    pub async fn get_address_analytics(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<Option<AddressAnalytics>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": {
                    "$nor": [
                        { "_id": { "$lt": start_index } },
                        { "_id": { "$gte": end_index } },
                    ],
                } },
                doc! { "$group": {
                    "_id": "$details.address",
                    "total_active_addresses": { "$sum": "total_active_addresses" },
                    "receiving_addresses": { "$sum": "receiving_addresses" },
                    "sending_addresses": { "$sum": "sending_addresses" },
                }},
                doc! { "$replaceWith": "$addresses" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Gathers output analytics.
    pub async fn get_output_analytics<O: OutputKind>(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<Option<OutputAnalytics>, Error> {
        if let Some(kind) = O::kind() {
            self.aggregate(
                vec![
                    doc! { "$match": {
                        "$nor": [
                            { "_id": { "$lt": start_index } },
                            { "_id": { "$gte": end_index } },
                        ]
                    } },
                    doc! { "$replaceWith": format!("$outputs.{}", kind) },
                    doc! { "$group" : {
                        "_id": null,
                        "count": { "$sum": "$count" },
                        "total_value": { "$sum": { "$toDecimal": "$total_value" } },
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
            .await
        } else {
            Ok(None)
        }
    }

    /// Gathers unspent output analytics.
    pub async fn get_unspent_output_analytics<O: OutputKind>(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<Option<OutputAnalytics>, Error> {
        {
            if let Some(kind) = O::kind() {
                self.aggregate(
                    vec![
                        doc! { "$match": {
                            "_id": ledger_index,
                        } },
                        doc! { "$replaceWith": format!("$unspent_outputs.{}", kind) },
                        doc! { "$group" : {
                            "_id": null,
                            "count": { "$sum": "$count" },
                            "total_value": { "$sum": { "$toDecimal": "$total_value" } },
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
                .await
            } else {
                Ok(None)
            }
        }
    }

    /// Gathers byte cost and storage deposit analytics.
    pub async fn get_storage_deposit_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<Option<StorageDepositAnalytics>, Error> {
        self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "_id": ledger_index,
                    } },
                    doc! { "$replaceWith": "$storage_deposits" },
                    doc! { "$group" : {
                        "_id": null,
                        "storage_deposit_return_count": { "$sum": "$storage_deposit_return_count" },
                        "storage_deposit_return_total_value": { "$sum": { "$toDecimal": "$storage_deposit_return_total_value" } },
                        "total_key_bytes": { "$sum": { "$toDecimal": "$total_key_bytes" } },
                        "total_data_bytes": { "$sum": { "$toDecimal": "$total_data_bytes" } },
                        "total_byte_cost": { "$sum": { "$toDecimal": "$total_byte_cost" } },
                    } },
                    doc! { "$project": {
                        "output_count": { "$first": "$all.output_count" },
                        "storage_deposit_return_count": { "$ifNull": [ { "$first": "$storage_deposit.return_count" }, 0 ] },
                        "storage_deposit_return_total_value": { 
                            "$toString": { "$ifNull": [ { "$first": "$storage_deposit.return_total_value" }, 0 ] } 
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
            .await
    }

    /// Gathers past-cone activity statistics for a given
    pub async fn get_milestone_activity(
        &self,
        index: MilestoneIndex,
    ) -> Result<Option<MilestoneActivityAnalytics>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": { "_id": index } },
                doc! { "$group": {
                    "_id": null,
                    "num_blocks": { "$sum": "$num_blocks" },
                    "num_tx_payload": { "$sum": "$num_tx_payload" },
                    "num_treasury_tx_payload": { "$sum": "$num_treasury_tx_payload" },
                    "num_milestone_payload": { "$sum": "$num_milestone_payload" },
                    "num_tagged_data_payload": { "$sum": "$num_tagged_data_payload" },
                    "num_no_payload": { "$sum": "$num_no_payload" },
                    "num_confirmed_tx": { "$sum": "$num_confirmed_tx" },
                    "num_conflicting_tx": { "$sum": "$num_conflicting_tx" },
                    "num_no_tx": { "$sum": "$num_no_tx" },
                } },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Gets the number of claimed tokens.
    pub async fn get_claimed_token_analytics(
        &self,
        index: MilestoneIndex,
    ) -> Result<Option<ClaimedTokensAnalytics>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": {
                    "_id": index,
                } },
                doc! { "$group": {
                    "_id": null,
                    "count": { "$sum": { "$toDecimal": "$count" } },
                } },
                doc! { "$project": {
                    "count": { "$toString": "$count" },
                } },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }
}
