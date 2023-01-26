// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Schema implementation for InfluxDb.
pub mod influx;

mod address_balance;
mod base_token;
mod block_activity;
mod daily_active_addresses;
mod ledger_outputs;
mod ledger_size;
mod output_activity;
mod unclaimed_tokens;
mod unlock_condition;

use std::fmt::Debug;

use influxdb::{InfluxDbWriteable, WriteQuery};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

pub use self::{
    address_balance::AddressAnalyticsResult,
    base_token::BaseTokenActivityAnalyticsResult,
    block_activity::{
        BlockActivityAnalyticsResult, PayloadActivityAnalyticsResult, TransactionActivityAnalyticsResult,
    },
    daily_active_addresses::DailyActiveAddressAnalyticsResult,
    ledger_outputs::LedgerOutputAnalyticsResult,
    ledger_size::LedgerSizeAnalyticsResult,
    output_activity::OutputActivityAnalyticsResult,
    unclaimed_tokens::UnclaimedTokenAnalyticsResult,
    unlock_condition::UnlockConditionAnalyticsResult,
};
use crate::types::{
    ledger::MilestoneIndexTimestamp, stardust::milestone::MilestoneTimestamp, tangle::ProtocolParameters,
};

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct PerMilestone<M> {
    pub at: MilestoneIndexTimestamp,
    pub inner: M,
}

impl<M> PerMilestone<M> {
    fn prepare_query(&self, name: impl Into<String>) -> WriteQuery {
        influxdb::Timestamp::from(self.at.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.at.milestone_index)
    }
}

/// Note: We will need this later, for example for daily active addresses.
#[allow(unused)]
#[allow(missing_docs)]
pub struct TimeInterval<M> {
    pub from: OffsetDateTime,
    pub to_exclusive: OffsetDateTime,
    pub inner: M,
}

impl<M> TimeInterval<M> {
    fn prepare_query(&self, name: impl Into<String>) -> WriteQuery {
        // We subtract 1 nanosecond to get the inclusive end of the time interval.
        let timestamp = self.to_exclusive - Duration::nanoseconds(1);
        influxdb::Timestamp::from(MilestoneTimestamp::from(timestamp)).into_query(name)
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Time(#[from] time::Error),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SyncAnalytics {
    pub sync_time: u64,
}

#[allow(missing_docs)]
pub enum Measurement {
    AddressActivity(PerMilestone<AddressAnalyticsResult>),
    BaseTokenActivity(PerMilestone<BaseTokenActivityAnalyticsResult>),
    BlockActivity(PerMilestone<BlockActivityAnalyticsResult>),
    DailyActiveAddresses(TimeInterval<DailyActiveAddressAnalyticsResult>),
    LedgerOutputs(PerMilestone<LedgerOutputAnalyticsResult>),
    LedgerSize(PerMilestone<LedgerSizeAnalyticsResult>),
    OutputActivity(PerMilestone<OutputActivityAnalyticsResult>),
    ProtocolParameters(PerMilestone<ProtocolParameters>),
    UnclaimedTokens(PerMilestone<UnclaimedTokenAnalyticsResult>),
    UnlockConditions(PerMilestone<UnlockConditionAnalyticsResult>),
}
