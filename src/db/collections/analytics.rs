// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

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
        ledger::{BlockMetadata, LedgerInclusionState},
        stardust::{
            block::{
                output::{AliasOutput, BasicOutput, FoundryOutput, NftOutput},
                Address, Block, Output, Payload,
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
#[allow(missing_docs)]
pub struct Analytics {
    pub addresses: AddressAnalytics,
    pub outputs: HashMap<String, OutputAnalytics>,
    pub unspent_outputs: HashMap<String, OutputAnalytics>,
    pub storage_deposits: StorageDepositAnalytics,
    pub claimed_tokens: ClaimedTokensAnalytics,
    pub milestone_activity: MilestoneActivityAnalytics,
}

impl Default for Analytics {
    fn default() -> Self {
        Self {
            addresses: Default::default(),
            outputs: [
                (BasicOutput::kind().unwrap().to_string(), Default::default()),
                (AliasOutput::kind().unwrap().to_string(), Default::default()),
                (NftOutput::kind().unwrap().to_string(), Default::default()),
                (FoundryOutput::kind().unwrap().to_string(), Default::default()),
            ]
            .into(),
            unspent_outputs: [
                (BasicOutput::kind().unwrap().to_string(), Default::default()),
                (AliasOutput::kind().unwrap().to_string(), Default::default()),
                (NftOutput::kind().unwrap().to_string(), Default::default()),
                (FoundryOutput::kind().unwrap().to_string(), Default::default()),
            ]
            .into(),
            storage_deposits: Default::default(),
            claimed_tokens: Default::default(),
            milestone_activity: Default::default(),
        }
    }
}

impl Analytics {
    /// Get a processor to update the analytics with new data.
    pub fn processor(self) -> AnalyticsProcessor {
        AnalyticsProcessor {
            analytics: self,
            addresses: Default::default(),
            sending_addresses: Default::default(),
            receiving_addresses: Default::default(),
            removed_outputs: Default::default(),
            removed_storage_deposits: Default::default(),
        }
    }
}

/// A processor for analytics which holds some state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnalyticsProcessor {
    analytics: Analytics,
    addresses: HashSet<Address>,
    sending_addresses: HashSet<Address>,
    receiving_addresses: HashSet<Address>,
    removed_outputs: HashMap<String, OutputAnalytics>,
    removed_storage_deposits: StorageDepositAnalytics,
}

impl AnalyticsProcessor {
    /// Process a batch of outputs.
    pub fn process_outputs<'a, I>(&mut self, outputs: I)
    where
        I: IntoIterator<Item = &'a OutputDocument>,
    {
        for output in outputs {
            if let Some(address) = output.details.address {
                self.addresses.insert(address);
                if output.metadata.spent_metadata.is_some() {
                    self.sending_addresses.insert(address);
                } else {
                    self.receiving_addresses.insert(address);
                }
            }
            let output_analytics = self
                .analytics
                .outputs
                .entry(output.output.kind().to_string())
                .or_default();
            output_analytics.count += 1;
            output_analytics.total_value += output.output.amount().0.into();

            let (unspent_output_analytics, storage_deposits) = if output.metadata.spent_metadata.is_some() {
                // To workaround spent outputs being processed first, we keep track of a separate set
                // of values which will be subtracted at the end.
                (
                    self.removed_outputs
                        .entry(output.output.kind().to_string())
                        .or_default(),
                    &mut self.removed_storage_deposits,
                )
            } else {
                (
                    self.analytics
                        .unspent_outputs
                        .entry(output.output.kind().to_string())
                        .or_default(),
                    &mut self.analytics.storage_deposits,
                )
            };
            unspent_output_analytics.count += 1;
            unspent_output_analytics.total_value += output.output.amount().0.into();
            storage_deposits.output_count += 1;
            storage_deposits.total_data_bytes += output.details.rent_structure.num_data_bytes.into();
            storage_deposits.total_key_bytes += output.details.rent_structure.num_key_bytes.into();
            match output.output {
                Output::Basic(BasicOutput {
                    storage_deposit_return_unlock_condition: Some(uc),
                    ..
                })
                | Output::Nft(NftOutput {
                    storage_deposit_return_unlock_condition: Some(uc),
                    ..
                }) => {
                    storage_deposits.storage_deposit_return_count += 1;
                    storage_deposits.storage_deposit_return_total_value += uc.amount.0.into();
                }
                _ => (),
            }
            // TODO: Claimed tokens
        }
    }

    /// Process a batch of outputs.
    pub fn process_blocks<'a, I>(&mut self, blocks: I)
    where
        I: IntoIterator<Item = (&'a Block, &'a BlockMetadata)>,
    {
        for (block, metadata) in blocks {
            self.analytics.milestone_activity.count += 1;
            match &block.payload {
                Some(payload) => match payload {
                    Payload::Transaction(_) => self.analytics.milestone_activity.transaction_count += 1,
                    Payload::Milestone(_) => self.analytics.milestone_activity.milestone_count += 1,
                    Payload::TreasuryTransaction(_) => {
                        self.analytics.milestone_activity.treasury_transaction_count += 1
                    }
                    Payload::TaggedData(_) => self.analytics.milestone_activity.tagged_data_count += 1,
                },
                None => self.analytics.milestone_activity.no_payload_count += 1,
            }
            match &metadata.inclusion_state {
                LedgerInclusionState::Conflicting => self.analytics.milestone_activity.conflicting_count += 1,
                LedgerInclusionState::Included => self.analytics.milestone_activity.confirmed_count += 1,
                LedgerInclusionState::NoTransaction => self.analytics.milestone_activity.no_transaction_count += 1,
            }
        }
    }

    /// Complete processing and return the analytics.
    pub fn finish(mut self) -> Analytics {
        self.analytics.addresses.total_active_addresses = self.addresses.len() as _;
        self.analytics.addresses.receiving_addresses = self.receiving_addresses.len() as _;
        self.analytics.addresses.sending_addresses = self.sending_addresses.len() as _;
        for (key, val) in self.removed_outputs {
            self.analytics.unspent_outputs.get_mut(&key).unwrap().count -= val.count;
            self.analytics.unspent_outputs.get_mut(&key).unwrap().total_value -= val.total_value;
        }
        self.analytics.storage_deposits.output_count -= self.removed_storage_deposits.output_count;
        self.analytics.storage_deposits.storage_deposit_return_count -=
            self.removed_storage_deposits.storage_deposit_return_count;
        self.analytics.storage_deposits.storage_deposit_return_total_value -=
            self.removed_storage_deposits.storage_deposit_return_total_value;
        self.analytics.storage_deposits.total_data_bytes -= self.removed_storage_deposits.total_data_bytes;
        self.analytics.storage_deposits.total_key_bytes -= self.removed_storage_deposits.total_key_bytes;
        self.analytics
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
    pub count: d128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MilestoneActivityAnalytics {
    /// The number of blocks referenced by a milestone.
    pub count: u32,
    /// The number of blocks referenced by a milestone that contain a payload.
    pub transaction_count: u32,
    /// The number of blocks containing a treasury transaction payload.
    pub treasury_transaction_count: u32,
    /// The number of blocks containing a milestone payload.
    pub milestone_count: u32,
    /// The number of blocks containing a tagged data payload.
    pub tagged_data_count: u32,
    /// The number of blocks referenced by a milestone that contain no payload.
    pub no_payload_count: u32,
    /// The number of blocks containing a confirmed transaction.
    pub confirmed_count: u32,
    /// The number of blocks containing a conflicting transaction.
    pub conflicting_count: u32,
    /// The number of blocks containing no transaction.
    pub no_transaction_count: u32,
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
                            .meta_field(Some("analytics".to_string()))
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
        Ok(
            match self
                .aggregate(
                    vec![
                        doc! { "$match": { "_id": milestone_index } },
                        doc! { "$replaceWith": "$analytics" },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
            {
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
            },
        )
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
                doc! { "$replaceWith": "$analytics.addresses" },
                doc! { "$group": {
                    "_id": null,
                    "total_active_addresses": { "$sum": "$total_active_addresses" },
                    "receiving_addresses": { "$sum": "$receiving_addresses" },
                    "sending_addresses": { "$sum": "$sending_addresses" },
                }},
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
                    doc! { "$replaceWith": format!("$analytics.outputs.{}", kind) },
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
                        doc! { "$replaceWith": format!("$analytics.unspent_outputs.{}", kind) },
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
        self.aggregate(
            vec![
                doc! { "$match": {
                    "_id": ledger_index,
                } },
                doc! { "$replaceWith": "$analytics.storage_deposits" },
                doc! { "$group" : {
                    "_id": null,
                    "output_count": { "$sum": "$output_count" },
                    "storage_deposit_return_count": { "$sum": "$storage_deposit_return_count" },
                    "storage_deposit_return_total_value": { "$sum": { "$toDecimal": "$storage_deposit_return_total_value"} },
                    "total_key_bytes": { "$sum": { "$toDecimal": "$total_key_bytes" } },
                    "total_data_bytes": { "$sum": { "$toDecimal": "$total_data_bytes" } },
                } },
                doc! { "$project": {
                    "output_count": 1,
                    "storage_deposit_return_count": 1,
                    "storage_deposit_return_total_value": { "$toString": "$storage_deposit_return_total_value" },
                    "total_key_bytes": { "$toString": "$total_key_bytes" },
                    "total_data_bytes": { "$toString": "$total_data_bytes" },
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
                doc! { "$replaceWith": "$analytics.milestone_activity" },
                doc! { "$group": {
                    "_id": null,
                    "count": { "$sum": "$count" },
                    "transaction_count": { "$sum": "$transaction_count" },
                    "treasury_transaction_count": { "$sum": "$treasury_transaction_count" },
                    "milestone_count": { "$sum": "$milestone_count" },
                    "tagged_data_count": { "$sum": "$tagged_data_count" },
                    "no_payload_count": { "$sum": "$num_nono_payload_count_payload" },
                    "confirmed_count": { "$sum": "$confirmed_count" },
                    "conflicting_count": { "$sum": "$conflicting_count" },
                    "no_transaction_count": { "$sum": "$no_transaction_count" },
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
                doc! { "$replaceWith": "$analytics.claimed_tokens" },
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
