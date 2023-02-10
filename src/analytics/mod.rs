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
        LedgerSizeAnalytics, OutputActivityMeasurement, TransactionDistributionMeasurement, UnclaimedTokenMeasurement,
        UnlockConditionMeasurement,
    },
    tangle::{BlockActivityMeasurement, MilestoneSizeMeasurement, ProtocolParamsMeasurement},
};
use crate::{
    db::influxdb::{AnalyticsChoice, InfluxDb},
    tangle::{BlockData, InputSource, Milestone},
    types::{
        ledger::{LedgerInclusionState, LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::{payload::TransactionEssence, Input, Payload},
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

mod influx;
mod ledger;
mod tangle;

/// Provides an API to access basic information used for analytics
pub(crate) trait AnalyticsContext {
    fn protocol_params(&self) -> &ProtocolParameters;

    fn at(&self) -> &MilestoneIndexTimestamp;
}

impl<'a, I: InputSource> AnalyticsContext for Milestone<'a, I> {
    fn protocol_params(&self) -> &ProtocolParameters {
        &self.protocol_params
    }

    fn at(&self) -> &MilestoneIndexTimestamp {
        &self.at
    }
}

#[allow(missing_docs)]
trait Analytics {
    type Measurement;
    fn begin_milestone(&mut self, ctx: &dyn AnalyticsContext);
    fn handle_transaction(
        &mut self,
        _consumed: &[LedgerSpent],
        _created: &[LedgerOutput],
        _ctx: &dyn AnalyticsContext,
    ) {
    }
    fn handle_block(&mut self, _block_data: &BlockData, _ctx: &dyn AnalyticsContext) {}
    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Self::Measurement>;
}

// This trait allows using the above implementation dynamically
trait DynAnalytics: Send {
    fn begin_milestone(&mut self, ctx: &dyn AnalyticsContext);
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext);
    fn handle_block(&mut self, block_data: &BlockData, ctx: &dyn AnalyticsContext);
    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Box<dyn PrepareQuery>>;
}

impl<T: Analytics + Send> DynAnalytics for T
where
    T::Measurement: 'static + PrepareQuery,
{
    fn begin_milestone(&mut self, ctx: &dyn AnalyticsContext) {
        Analytics::begin_milestone(self, ctx)
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext) {
        Analytics::handle_transaction(self, consumed, created, ctx)
    }

    fn handle_block(&mut self, block_data: &BlockData, ctx: &dyn AnalyticsContext) {
        Analytics::handle_block(self, block_data, ctx)
    }

    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Box<dyn PrepareQuery>> {
        Analytics::end_milestone(self, ctx).map(|r| Box::new(r) as _)
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
            AnalyticsChoice::BaseTokenActivity => Box::<BaseTokenActivityMeasurement>::default() as _,
            AnalyticsChoice::BlockActivity => Box::<BlockActivityMeasurement>::default() as _,
            AnalyticsChoice::DailyActiveAddresses => Box::new(AddressActivityAnalytics::init(
                OffsetDateTime::now_utc().date().midnight().assume_utc(),
                time::Duration::days(1),
                unspent_outputs,
            )) as _,
            AnalyticsChoice::LedgerOutputs => Box::new(LedgerOutputMeasurement::init(unspent_outputs)) as _,
            AnalyticsChoice::LedgerSize => {
                Box::new(LedgerSizeAnalytics::init(protocol_params.clone(), unspent_outputs)) as _
            }
            AnalyticsChoice::MilestoneSize => Box::<MilestoneSizeMeasurement>::default() as _,
            AnalyticsChoice::OutputActivity => Box::<OutputActivityMeasurement>::default() as _,
            AnalyticsChoice::ProtocolParameters => Box::<ProtocolParamsMeasurement>::default() as _,
            AnalyticsChoice::TransactionDistribution => Box::<TransactionDistributionMeasurement>::default() as _,
            AnalyticsChoice::UnclaimedTokens => Box::new(UnclaimedTokenMeasurement::init(unspent_outputs)) as _,
            AnalyticsChoice::UnlockConditions => Box::new(UnlockConditionMeasurement::init(unspent_outputs)) as _,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("missing created output ({output_id}) in milestone {milestone_index}")]
    MissingLedgerOutput {
        output_id: String,
        milestone_index: MilestoneIndex,
    },
    #[error("missing consumed output ({output_id}) in milestone {milestone_index}")]
    MissingLedgerSpent {
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
            analytic.0.begin_milestone(self)
        }
    }

    fn handle_block(&self, analytics: &mut [Analytic], block_data: &BlockData) -> eyre::Result<()> {
        if block_data.metadata.inclusion_state == LedgerInclusionState::Included {
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
                            .ok_or(AnalyticsError::MissingLedgerSpent {
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
                    analytic.0.handle_transaction(&consumed, &created, self);
                }
            }
        }
        for analytic in analytics.iter_mut() {
            analytic.0.handle_block(block_data, self);
        }
        Ok(())
    }

    async fn end_milestone(&self, analytics: &mut [Analytic], influxdb: &InfluxDb) -> eyre::Result<()> {
        for measurement in analytics
            .iter_mut()
            .filter_map(|analytic| analytic.0.end_milestone(self))
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

#[derive(Clone, Debug)]
#[allow(missing_docs)]
struct PerMilestone<M> {
    at: MilestoneIndexTimestamp,
    inner: M,
}

/// Note: We will need this later, for example for daily active addresses.
#[allow(unused)]
#[allow(missing_docs)]
struct TimeInterval<M> {
    from: OffsetDateTime,
    to_exclusive: OffsetDateTime,
    inner: M,
}

#[cfg(test)]
mod test {
    use super::AnalyticsContext;
    use crate::types::{ledger::MilestoneIndexTimestamp, tangle::ProtocolParameters};

    pub(crate) struct TestContext {
        pub(crate) at: MilestoneIndexTimestamp,
        pub(crate) params: ProtocolParameters,
    }

    impl AnalyticsContext for TestContext {
        fn protocol_params(&self) -> &ProtocolParameters {
            &self.params
        }

        fn at(&self) -> &MilestoneIndexTimestamp {
            &self.at
        }
    }
}
