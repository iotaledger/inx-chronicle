// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Various analytics that give insight into the usage of the tangle.

use futures::TryStreamExt;
use thiserror::Error;
use time::OffsetDateTime;

use self::{
    influx::PrepareQuery,
    ledger::{
        AddressActivityAnalytics, AddressBalancesAnalytics, BaseTokenActivityMeasurement, LedgerOutputMeasurement,
        LedgerSizeAnalytics, OutputActivityMeasurement, UnclaimedTokenMeasurement, UnlockConditionMeasurement,
    },
    protocol_params::ProtocolParamsMeasurement,
    tangle::{BlockActivityMeasurement, MilestoneSizeMeasurement},
};
use crate::{
    db::influxdb::{AnalyticsChoice, InfluxDb},
    tangle::{BlockData, InputSource, Milestone},
    types::{
        ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::{payload::TransactionEssence, Input, Payload},
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

pub mod influx;
pub mod ledger;
mod protocol_params;
pub mod tangle;

#[allow(missing_docs)]
pub(crate) trait Analytics {
    type Measurement: PrepareQuery;
    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp, params: &ProtocolParameters);
    fn handle_transaction(&mut self, _consumed: &[LedgerSpent], _created: &[LedgerOutput]) {}
    fn handle_block(&mut self, _block_data: &BlockData) {}
    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement>;
}

// This trait allows using the above implementation dynamically
trait DynAnalytics: Send {
    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp, params: &ProtocolParameters);
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]);
    fn handle_block(&mut self, block_data: &BlockData);
    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Box<dyn PrepareQuery>>;
}

impl<T: Analytics + Send> DynAnalytics for T
where
    T::Measurement: 'static,
{
    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp, params: &ProtocolParameters) {
        Analytics::begin_milestone(self, at, params)
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        Analytics::handle_transaction(self, consumed, created)
    }

    fn handle_block(&mut self, block_data: &BlockData) {
        Analytics::handle_block(self, block_data)
    }

    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Box<dyn PrepareQuery>> {
        Analytics::end_milestone(self, at).map(|r| Box::new(r) as _)
    }
}

#[allow(missing_docs)]
pub struct Analytic(Box<dyn DynAnalytics>);

impl Analytic {
    /// Init an analytic from a choice and ledger state.
    pub fn init<'a>(
        choice: &AnalyticsChoice,
        protocol_params: &ProtocolParameters,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
    ) -> Self {
        Self(match choice {
            AnalyticsChoice::AddressBalance => Box::new(AddressBalancesAnalytics::init(unspent_outputs)) as _,
            AnalyticsChoice::BaseTokenActivity => Box::new(BaseTokenActivityMeasurement::default()) as _,
            AnalyticsChoice::BlockActivity => Box::new(BlockActivityMeasurement::default()) as _,
            AnalyticsChoice::DailyActiveAddresses => Box::new(AddressActivityAnalytics::init(
                OffsetDateTime::now_utc().date().midnight().assume_utc(),
                time::Duration::days(1),
                unspent_outputs,
            )) as _,
            AnalyticsChoice::LedgerOutputs => Box::new(LedgerOutputMeasurement::init(unspent_outputs)) as _,
            AnalyticsChoice::LedgerSize => {
                Box::new(LedgerSizeAnalytics::init(protocol_params.clone(), unspent_outputs)) as _
            }
            AnalyticsChoice::MilestoneSize => Box::new(MilestoneSizeMeasurement::default()) as _,
            AnalyticsChoice::OutputActivity => Box::new(OutputActivityMeasurement::default()) as _,
            AnalyticsChoice::ProtocolParameters => Box::new(ProtocolParamsMeasurement::default()) as _,
            AnalyticsChoice::UnclaimedTokens => Box::new(UnclaimedTokenMeasurement::init(unspent_outputs)) as _,
            AnalyticsChoice::UnlockConditions => Box::new(UnlockConditionMeasurement::init(unspent_outputs)) as _,
        })
    }
}

impl DynAnalytics for Analytic {
    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp, params: &ProtocolParameters) {
        self.0.begin_milestone(at, params)
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        self.0.handle_transaction(consumed, created)
    }

    fn handle_block(&mut self, block_data: &BlockData) {
        self.0.handle_block(block_data)
    }

    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Box<dyn PrepareQuery>> {
        self.0.end_milestone(at)
    }
}

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("missing output ({output_id}) in milestone {milestone_index}")]
    MissingLedgerOutput {
        output_id: String,
        milestone_index: MilestoneIndex,
    },
}

impl<'a, I: InputSource> Milestone<'a, I> {
    /// Update a list of analytics with this milestone
    pub async fn update_analytics(&self, analytics: &mut [Analytic], influxdb: &InfluxDb) -> eyre::Result<()> {
        let mut cone_stream = self.cone_stream().await?;

        self.begin_milestone(analytics);

        while let Some(block_data) = cone_stream.try_next().await? {
            self.handle_block(analytics, &block_data)?;
        }

        self.end_milestone(analytics, influxdb).await?;

        Ok(())
    }

    fn begin_milestone(&self, analytics: &mut [Analytic]) {
        for analytic in analytics {
            analytic.begin_milestone(self.at, &self.protocol_params)
        }
    }

    fn handle_block(&self, analytics: &mut [Analytic], block_data: &BlockData) -> eyre::Result<()> {
        if let Some(Payload::Transaction(payload)) = &block_data.block.payload {
            let TransactionEssence::Regular { inputs, outputs, .. } = &payload.essence;
            let consumed = inputs
                .iter()
                .filter_map(|input| match input {
                    Input::Utxo(output_id) => Some(output_id),
                    _ => None,
                })
                .map(|output_id| {
                    Ok(self
                        .ledger_updates()
                        .get_consumed(output_id)
                        .ok_or(AnalyticsError::MissingLedgerOutput {
                            output_id: output_id.to_hex(),
                            milestone_index: block_data.metadata.referenced_by_milestone_index,
                        })?
                        .clone())
                })
                .collect::<eyre::Result<Vec<_>>>()?;
            let created = outputs
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    let output_id = (payload.transaction_id, index as _).into();
                    Ok(self
                        .ledger_updates()
                        .get_created(&output_id)
                        .ok_or(AnalyticsError::MissingLedgerOutput {
                            output_id: output_id.to_hex(),
                            milestone_index: block_data.metadata.referenced_by_milestone_index,
                        })?
                        .clone())
                })
                .collect::<eyre::Result<Vec<_>>>()?;
            for analytic in analytics.iter_mut() {
                analytic.handle_transaction(&consumed, &created);
            }
        }
        for analytic in analytics.iter_mut() {
            analytic.handle_block(block_data);
        }
        Ok(())
    }

    async fn end_milestone(&self, analytics: &mut [Analytic], influxdb: &InfluxDb) -> eyre::Result<()> {
        for measurement in analytics
            .iter_mut()
            .filter_map(|analytic| analytic.end_milestone(self.at))
        {
            influxdb.insert_measurement(measurement).await?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct SyncAnalytics {
    pub sync_time: u64,
}
