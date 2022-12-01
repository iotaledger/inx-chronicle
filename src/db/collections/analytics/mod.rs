// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Schema implementation for InfluxDb.
pub mod influx;

mod address_activity;
mod address_balance;
mod base_token;
mod block_activity;
mod ledger_outputs;
mod ledger_size;
mod output_activity;
mod unclaimed_tokens;

use std::fmt::Debug;

pub use address_activity::AddressActivityAnalytics;
pub use address_balance::AddressAnalytics;
use async_trait::async_trait;
pub use base_token::BaseTokenActivityAnalytics;
use decimal::d128;
use futures::TryFutureExt;
pub use ledger_outputs::LedgerOutputAnalytics;
pub use ledger_size::LedgerSizeAnalytics;
use mongodb::{bson::doc, error::Error};
pub use output_activity::OutputActivityAnalytics;
use serde::{Deserialize, Serialize};
pub use unclaimed_tokens::UnclaimedTokenAnalytics;
pub use block_activity::BlockActivityAnalytics;

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
#[allow(unused)]
pub struct TimeInterval<M> {
    milestone_timestamp: MilestoneTimestamp,
    measurement: M,
}

pub trait Measurement: Debug + Send + Sync {
    fn into_write_query(&self) -> influxdb::WriteQuery;
}

#[async_trait]
pub trait Analytic: Debug + Send + Sync {
    /// Note that we return an `Option` so that we don't always have to produce a metric for a given milestone. This is
    /// useful for values that don't change often, or if we want to aggregate over time intervals, for example. We also
    /// call this method on a mutable reference of `self` so that each analytic can decide if it wants to manage
    /// internal state.
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Box<dyn Measurement>, Error>>;
}

pub fn all_analytics() -> Vec<Box<dyn Analytic>> {
    vec![
        Box::new(AddressActivityAnalytics),
        Box::new(AddressAnalytics),
        Box::new(BaseTokenActivityAnalytics),
        Box::new(BlockActivityAnalytics),
        Box::new(LedgerOutputAnalytics),
        Box::new(LedgerSizeAnalytics),
        Box::new(UnclaimedTokenAnalytics),
    ]
}

/// Holds analytics about stardust data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Analytics {
    // pub address_activity: AddressActivityAnalyticsResult,
    // pub addresses: AddressAnalytics,
    // pub base_token: BaseTokenActivityAnalytics,
    // pub ledger_outputs: LedgerOutputAnalytics,
    // pub output_activity: OutputActivityAnalytics,
    // pub ledger_size: LedgerSizeAnalytics,
    // pub unclaimed_tokens: UnclaimedTokenAnalytics,
    // pub block_activity: BlockActivityAnalytics,
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
            // addresses,
            // ledger_outputs,
            // output_activity,
            // ledger_size,
            // unclaimed_tokens,
            unlock_conditions,
            // address_activity,
            // base_token,
            // block_activity,
            protocol_params,
        ) = tokio::try_join!(
            // output_collection.get_address_analytics(milestone_index),
            // output_collection.get_ledger_output_analytics(milestone_index),
            // output_collection.get_output_activity_analytics(milestone_index),
            // output_collection.get_ledger_size_analytics(milestone_index),
            // output_collection.get_unclaimed_token_analytics(milestone_index),
            output_collection.get_unlock_condition_analytics(milestone_index),
            // output_collection.get_address_activity_analytics(milestone_index),
            // output_collection.get_base_token_activity_analytics(milestone_index),
            // block_collection.get_block_activity_analytics(milestone_index),
            protocol_param_collection
                .get_protocol_parameters_for_milestone_index(milestone_index)
                .and_then(|p| async move { Ok(p.map(|p| p.parameters)) }),
        )?;

        Ok(Analytics {
            // address_activity,
            // addresses,
            // base_token,
            // ledger_outputs,
            // output_activity,
            // ledger_size,
            // unclaimed_tokens,
            // block_activity,
            unlock_conditions,
            protocol_params,
        })
    }
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
pub struct SyncAnalytics {
    pub sync_time: u64,
}
