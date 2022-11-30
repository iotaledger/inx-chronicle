// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Schema implementation for InfluxDb.
pub mod influx;

mod address_activity;

use std::fmt::Debug;

use async_trait::async_trait;
use decimal::d128;
use futures::TryFutureExt;
use influxdb::{InfluxDbWriteable, WriteQuery};
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

pub use address_activity::AddressActivityAnalytics;

use super::{BlockCollection, OutputCollection, ProtocolUpdateCollection};
use crate::{
    db::MongoDb,
    types::{
        stardust::milestone::MilestoneTimestamp,
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

#[derive(Debug)]
pub struct PerMilestone<M> {
    milestone_timestamp: MilestoneTimestamp,
    milestone_index: MilestoneIndex,
    measurement: M,
}

/// TODO: We will need this later.
pub struct TimeInterval<M> {
    milestone_timestamp: MilestoneTimestamp,
    measurement: M,
}

pub trait Measurement: Debug + Send + Sync {
    fn into_write_query(&self) -> influxdb::WriteQuery;
}

#[async_trait]
pub trait Analytic: Debug + Send + Sync {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Box<dyn Measurement>, Error>>;
}



/// Holds analytics about stardust data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Analytics {
    // pub address_activity: AddressActivityAnalyticsResult,
    pub addresses: AddressAnalytics,
    pub base_token: BaseTokenActivityAnalytics,
    pub ledger_outputs: LedgerOutputAnalytics,
    pub output_activity: OutputActivityAnalytics,
    pub ledger_size: LedgerSizeAnalytics,
    pub unclaimed_tokens: UnclaimedTokensAnalytics,
    pub block_activity: BlockActivityAnalytics,
    pub unlock_conditions: UnlockConditionAnalytics,
    pub protocol_params: Option<ProtocolParameters>,
}

impl MongoDb {
    /// Gets selected analytics for a given milestone index, fetching the data from collections.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_analytics(
        &self,
        analytics: &mut Vec<Box<dyn Analytic>>,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Vec<Box<dyn Measurement>>, Error> {
        let mut res = Vec::new();
        for a in analytics {
            if let Some(m) = a.get_measurement(self, milestone_index, milestone_timestamp).await {
                res.push(m?);
            }
        }
        Ok(res)
    }

    #[deprecated]
    /// Gets all analytics for a milestone index, fetching the data from the collections.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_all_analytics(&self, milestone_index: MilestoneIndex) -> Result<Analytics, Error> {
        let output_collection = self.collection::<OutputCollection>();
        let block_collection = self.collection::<BlockCollection>();
        let protocol_param_collection = self.collection::<ProtocolUpdateCollection>();

        let (
            addresses,
            ledger_outputs,
            output_activity,
            ledger_size,
            unclaimed_tokens,
            unlock_conditions,
            //address_activity,
            base_token,
            block_activity,
            protocol_params,
        ) = tokio::try_join!(
            output_collection.get_address_analytics(milestone_index),
            output_collection.get_ledger_output_analytics(milestone_index),
            output_collection.get_output_activity_analytics(milestone_index),
            output_collection.get_ledger_size_analytics(milestone_index),
            output_collection.get_unclaimed_token_analytics(milestone_index),
            output_collection.get_unlock_condition_analytics(milestone_index),
            //output_collection.get_address_activity_analytics(milestone_index),
            output_collection.get_base_token_activity_analytics(milestone_index),
            block_collection.get_block_activity_analytics(milestone_index),
            protocol_param_collection
                .get_protocol_parameters_for_milestone_index(milestone_index)
                .and_then(|p| async move { Ok(p.map(|p| p.parameters)) }),
        )?;

        Ok(Analytics {
            // address_activity,
            addresses,
            base_token,
            ledger_outputs,
            output_activity,
            ledger_size,
            unclaimed_tokens,
            block_activity,
            unlock_conditions,
            protocol_params,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AddressAnalytics {
    pub address_with_balance_count: u64,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UnlockConditionAnalytics {
    pub timelock_count: u64,
    pub timelock_value: d128,
    pub expiration_count: u64,
    pub expiration_value: d128,
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_value: d128,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerOutputAnalytics {
    pub basic_count: u64,
    pub basic_value: d128,
    pub alias_count: u64,
    pub alias_value: d128,
    pub foundry_count: u64,
    pub foundry_value: d128,
    pub nft_count: u64,
    pub nft_value: d128,
    pub treasury_count: u64,
    pub treasury_value: d128,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerSizeAnalytics {
    pub total_storage_deposit_value: d128,
    pub total_key_bytes: d128,
    pub total_data_bytes: d128,
}

#[allow(missing_docs)]
impl LedgerSizeAnalytics {
    pub fn total_byte_cost(&self, protocol_params: &ProtocolParameters) -> d128 {
        let rent_structure = protocol_params.rent_structure;
        d128::from(rent_structure.v_byte_cost)
            * ((self.total_key_bytes * d128::from(rent_structure.v_byte_factor_key as u32))
                + (self.total_data_bytes * d128::from(rent_structure.v_byte_factor_data as u32)))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UnclaimedTokensAnalytics {
    pub unclaimed_count: u64,
    pub unclaimed_value: d128,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(default)]
pub struct OutputActivityAnalytics {
    pub alias: AliasActivityAnalytics,
    pub nft: NftActivityAnalytics,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(default)]
pub struct AliasActivityAnalytics {
    pub created_count: u64,
    pub governor_changed_count: u64,
    pub state_changed_count: u64,
    pub destroyed_count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(default)]
pub struct NftActivityAnalytics {
    pub created_count: u64,
    pub transferred_count: u64,
    pub destroyed_count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BaseTokenActivityAnalytics {
    pub transferred_value: d128,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(default)]
pub struct FoundryActivityAnalytics {
    pub created_count: u64,
    pub transferred_count: u64,
    pub destroyed_count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockActivityAnalytics {
    pub payload: PayloadActivityAnalytics,
    pub transaction: TransactionActivityAnalytics,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct PayloadActivityAnalytics {
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
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TransactionActivityAnalytics {
    /// The number of blocks containing a confirmed transaction.
    pub confirmed_count: u32,
    /// The number of blocks containing a conflicting transaction.
    pub conflicting_count: u32,
    /// The number of blocks containing no transaction.
    pub no_transaction_count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SyncAnalytics {
    pub sync_time: u64,
}
