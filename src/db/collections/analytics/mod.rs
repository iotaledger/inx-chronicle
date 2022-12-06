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
use influxdb::{InfluxDbWriteable, WriteQuery};
pub use ledger_outputs::LedgerOutputAnalytics;
pub use ledger_size::LedgerSizeAnalytics;
use mongodb::{bson::doc, error::Error};
pub use output_activity::OutputActivityAnalytics;
pub use protocol_parameters::ProtocolParametersAnalytics;
use serde::{Deserialize, Serialize};
pub use unclaimed_tokens::UnclaimedTokenAnalytics;
pub use unlock_condition::UnlockConditionAnalytics;

use self::{
    address_activity::AddressActivityAnalyticsResult, address_balance::AddressAnalyticsResult,
    base_token::BaseTokenActivityAnalyticsResult, block_activity::BlockActivityAnalyticsResult,
    ledger_outputs::LedgerOutputAnalyticsResult, ledger_size::LedgerSizeAnalyticsResult,
    output_activity::OutputActivityAnalyticsResult, unclaimed_tokens::UnclaimedTokenAnalyticsResult,
    unlock_condition::UnlockConditionAnalyticsResult,
};
use crate::{
    db::MongoDb,
    types::{
        stardust::milestone::MilestoneTimestamp,
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct PerMilestone<M> {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub inner: M,
}

impl<M> PerMilestone<M> {
    fn prepare_query(&self, name: impl Into<String>) -> WriteQuery {
        influxdb::Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
    }
}

/// TODO: We will need this later.
#[allow(unused)]
#[allow(missing_docs)]
pub struct TimeInterval<M> {
    milestone_timestamp: MilestoneTimestamp,
    measurement: M,
}

#[async_trait]
/// A common trait for all analytics.
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
    ) -> Option<Result<Measurement, Error>>;
}

/// Returns a list of trait objects for all analytics.
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SyncAnalytics {
    pub sync_time: u64,
}

#[allow(missing_docs)]
pub enum Measurement {
    AddressActivityAnalytics(PerMilestone<AddressActivityAnalyticsResult>),
    AddressAnalytics(PerMilestone<AddressAnalyticsResult>),
    BaseTokenActivity(PerMilestone<BaseTokenActivityAnalyticsResult>),
    BlockAnalytics(PerMilestone<BlockActivityAnalyticsResult>),
    LedgerOutputAnalytics(PerMilestone<LedgerOutputAnalyticsResult>),
    LedgerSizeAnalytics(PerMilestone<LedgerSizeAnalyticsResult>),
    OutputActivityAnalytics(PerMilestone<OutputActivityAnalyticsResult>),
    ProtocolParameters(PerMilestone<ProtocolParameters>),
    UnclaimedTokenAnalytics(PerMilestone<UnclaimedTokenAnalyticsResult>),
    UnlockConditionAnalytics(PerMilestone<UnlockConditionAnalyticsResult>),
}
