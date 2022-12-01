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
mod protocol_parameters;
mod unclaimed_tokens;
mod unlock_condition;

use std::fmt::Debug;

pub use address_activity::AddressActivityAnalytics;
pub use address_balance::AddressAnalytics;
use async_trait::async_trait;
pub use base_token::BaseTokenActivityAnalytics;
pub use block_activity::BlockActivityAnalytics;
pub use ledger_outputs::LedgerOutputAnalytics;
pub use ledger_size::LedgerSizeAnalytics;
use mongodb::{bson::doc, error::Error};
pub use output_activity::OutputActivityAnalytics;
pub use protocol_parameters::ProtocolParametersAnalytics;
use serde::{Deserialize, Serialize};
pub use unclaimed_tokens::UnclaimedTokenAnalytics;
pub use unlock_condition::UnlockConditionAnalytics;

use crate::{
    db::MongoDb,
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
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
        Box::new(OutputActivityAnalytics),
        Box::new(UnclaimedTokenAnalytics),
        Box::new(UnlockConditionAnalytics),
        Box::new(ProtocolParametersAnalytics),
    ]
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

    /// Gets all analytics for a milestone index, fetching the data from the collections.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_all_analytics(&self, milestone_index: MilestoneIndex) -> Result<(), Error> {
        todo!() //
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SyncAnalytics {
    pub sync_time: u64,
}
