// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use decimal::d128;
#[cfg(feature = "inx")]
use influxdb::{InfluxDbWriteable, Timestamp};
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use super::{BlockCollection, OutputCollection, OutputKind};
#[cfg(feature = "inx")]
use crate::db::influxdb::{InfluxDb, InfluxDbMeasurement};
use crate::{
    db::MongoDb,
    types::{
        ledger::{BlockMetadata, LedgerInclusionState, LedgerOutput, LedgerSpent},
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

#[cfg(feature = "inx")]
impl InfluxDb {
    /// Insert all gathered analytics.
    pub async fn insert_all_analytics(
        &self,
        milestone_timestamp: MilestoneTimestamp,
        milestone_index: MilestoneIndex,
        mut analytics: Analytics,
    ) -> Result<(), influxdb::Error> {
        self.insert(AddressAnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            analytics: analytics.addresses,
        })
        .await?;
        for (kind, outputs) in analytics.outputs.drain() {
            self.insert(OutputAnalyticsSchema {
                milestone_timestamp,
                milestone_index,
                kind,
                analytics: outputs,
            })
            .await?;
        }
        for (kind, outputs) in analytics.unspent_outputs.drain() {
            self.insert(OutputAnalyticsSchema {
                milestone_timestamp,
                milestone_index,
                kind,
                analytics: outputs,
            })
            .await?;
        }
        self.insert(StorageDepositAnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            analytics: analytics.storage_deposits,
        })
        .await?;
        self.insert(ClaimedTokensAnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            analytics: analytics.claimed_tokens,
        })
        .await?;
        self.insert(MilestoneActivityAnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            analytics: analytics.milestone_activity,
        })
        .await?;
        Ok(())
    }
}

impl MongoDb {
    /// Gets all analytics for a milestone index, fetching the data from the collections.
    pub async fn get_all_analytics(&self, milestone_index: MilestoneIndex) -> Result<Analytics, Error> {
        let output_collection = self.collection::<OutputCollection>();
        let block_collection = self.collection::<BlockCollection>();
        let addresses = output_collection
            .get_address_analytics(milestone_index, milestone_index + 1)
            .await?;
        let mut outputs = HashMap::new();
        outputs.insert(
            BasicOutput::kind().unwrap().to_string(),
            output_collection
                .get_output_analytics::<BasicOutput>(milestone_index, milestone_index + 1)
                .await?,
        );
        outputs.insert(
            AliasOutput::kind().unwrap().to_string(),
            output_collection
                .get_output_analytics::<AliasOutput>(milestone_index, milestone_index + 1)
                .await?,
        );
        outputs.insert(
            NftOutput::kind().unwrap().to_string(),
            output_collection
                .get_output_analytics::<NftOutput>(milestone_index, milestone_index + 1)
                .await?,
        );
        outputs.insert(
            FoundryOutput::kind().unwrap().to_string(),
            output_collection
                .get_output_analytics::<FoundryOutput>(milestone_index, milestone_index + 1)
                .await?,
        );
        let mut unspent_outputs = HashMap::new();
        unspent_outputs.insert(
            BasicOutput::kind().unwrap().to_string(),
            output_collection
                .get_unspent_output_analytics::<BasicOutput>(milestone_index)
                .await?,
        );
        unspent_outputs.insert(
            AliasOutput::kind().unwrap().to_string(),
            output_collection
                .get_unspent_output_analytics::<AliasOutput>(milestone_index)
                .await?,
        );
        unspent_outputs.insert(
            NftOutput::kind().unwrap().to_string(),
            output_collection
                .get_unspent_output_analytics::<NftOutput>(milestone_index)
                .await?,
        );
        unspent_outputs.insert(
            FoundryOutput::kind().unwrap().to_string(),
            output_collection
                .get_unspent_output_analytics::<FoundryOutput>(milestone_index)
                .await?,
        );
        let storage_deposits = output_collection.get_storage_deposit_analytics(milestone_index).await?;
        let claimed_tokens = output_collection.get_claimed_token_analytics(milestone_index).await?;
        let milestone_activity = block_collection
            .get_milestone_activity_analytics(milestone_index)
            .await?;
        Ok(Analytics {
            addresses,
            outputs,
            unspent_outputs,
            storage_deposits,
            claimed_tokens,
            milestone_activity,
        })
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
    /// Process a batch of created outputs.
    pub fn process_created_outputs<'a, I>(&mut self, outputs: I)
    where
        I: IntoIterator<Item = &'a LedgerOutput>,
    {
        for output in outputs {
            self.process_output(output, false);
        }
    }

    /// Process a batch of consumed outputs.
    pub fn process_consumed_outputs<'a, I>(&mut self, outputs: I)
    where
        I: IntoIterator<Item = &'a LedgerSpent>,
    {
        for output in outputs {
            self.process_output(&output.output, true);
        }
    }

    fn process_output(&mut self, output: &LedgerOutput, is_spent: bool) {
        if let Some(&address) = output.output.owning_address() {
            self.addresses.insert(address);
            if is_spent {
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

        let (unspent_output_analytics, storage_deposits) = if is_spent {
            // Spent outputs that were created by the genesis are claimed.
            if output.booked.milestone_index == 0 {
                self.analytics.claimed_tokens.count += 1;
                self.analytics.claimed_tokens.total_amount += output.output.amount().0.into();
            }
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
        storage_deposits.total_data_bytes += output.rent_structure.num_data_bytes.into();
        storage_deposits.total_key_bytes += output.rent_structure.num_key_bytes.into();
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
    }

    /// Process a batch of blocks.
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

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct AddressAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: AddressAnalytics,
}

#[cfg(feature = "inx")]
impl InfluxDbWriteable for AddressAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_field("total_active_addresses", self.analytics.total_active_addresses)
            .add_field("receiving_addresses", self.analytics.receiving_addresses)
            .add_field("sending_addresses", self.analytics.sending_addresses)
    }
}

#[cfg(feature = "inx")]
impl InfluxDbMeasurement for AddressAnalyticsSchema {
    const NAME: &'static str = "stardust_addresses";
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputAnalytics {
    pub count: u64,
    pub total_value: d128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub kind: String,
    pub analytics: OutputAnalytics,
}

#[cfg(feature = "inx")]
impl InfluxDbWriteable for OutputAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_tag("kind", self.kind)
            .add_field("count", self.analytics.count)
            .add_field(
                "total_value",
                self.analytics.total_value.to_string().parse::<u64>().unwrap(),
            )
    }
}

#[cfg(feature = "inx")]
impl InfluxDbMeasurement for OutputAnalyticsSchema {
    const NAME: &'static str = "stardust_outputs";
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StorageDepositAnalytics {
    pub output_count: u64,
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_total_value: d128,
    pub total_key_bytes: d128,
    pub total_data_bytes: d128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StorageDepositAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: StorageDepositAnalytics,
}

#[cfg(feature = "inx")]
impl InfluxDbWriteable for StorageDepositAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_field("output_count", self.analytics.output_count)
            .add_field(
                "storage_deposit_return_count",
                self.analytics.storage_deposit_return_count,
            )
            .add_field(
                "storage_deposit_return_total_value",
                self.analytics
                    .storage_deposit_return_total_value
                    .to_string()
                    .parse::<u64>()
                    .unwrap(),
            )
            .add_field(
                "total_key_bytes",
                self.analytics.total_key_bytes.to_string().parse::<u64>().unwrap(),
            )
            .add_field(
                "total_data_bytes",
                self.analytics.total_data_bytes.to_string().parse::<u64>().unwrap(),
            )
    }
}

#[cfg(feature = "inx")]
impl InfluxDbMeasurement for StorageDepositAnalyticsSchema {
    const NAME: &'static str = "stardust_storage_deposits";
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
    pub total_amount: d128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClaimedTokensAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: ClaimedTokensAnalytics,
}

#[cfg(feature = "inx")]
impl InfluxDbWriteable for ClaimedTokensAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_field("count", self.analytics.count.to_string().parse::<u64>().unwrap())
    }
}

#[cfg(feature = "inx")]
impl InfluxDbMeasurement for ClaimedTokensAnalyticsSchema {
    const NAME: &'static str = "stardust_claimed_tokens";
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MilestoneActivityAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: MilestoneActivityAnalytics,
}

#[cfg(feature = "inx")]
impl InfluxDbWriteable for MilestoneActivityAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_field("count", self.analytics.count)
            .add_field("transaction_count", self.analytics.transaction_count)
            .add_field("treasury_transaction_count", self.analytics.treasury_transaction_count)
            .add_field("milestone_count", self.analytics.milestone_count)
            .add_field("tagged_data_count", self.analytics.tagged_data_count)
            .add_field("no_payload_count", self.analytics.no_payload_count)
            .add_field("confirmed_count", self.analytics.confirmed_count)
            .add_field("conflicting_count", self.analytics.conflicting_count)
            .add_field("no_transaction_count", self.analytics.no_transaction_count)
    }
}

#[cfg(feature = "inx")]
impl InfluxDbMeasurement for MilestoneActivityAnalyticsSchema {
    const NAME: &'static str = "stardust_milestone_activity";
}
