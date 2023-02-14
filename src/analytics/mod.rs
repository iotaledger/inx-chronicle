// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Various analytics that give insight into the usage of the tangle.

use futures::TryStreamExt;
use thiserror::Error;

use self::{
    influx::PrepareQuery,
    ledger::{
        AddressActivityAnalytics, AddressBalancesAnalytics, BaseTokenActivityMeasurement,
        DailyAddressActivityMeasurement, LedgerOutputMeasurement, LedgerSizeAnalytics, OutputActivityMeasurement,
        TransactionSizeMeasurement, UnclaimedTokenMeasurement, UnlockConditionMeasurement,
    },
    tangle::{BlockActivityMeasurement, MilestoneSizeMeasurement, ProtocolParamsMeasurement},
};
use crate::{
    db::{
        influxdb::{config::DailyAnalyticsChoice, AnalyticsChoice, InfluxDb},
        MongoDb,
    },
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
    PerMilestone<T::Measurement>: 'static + PrepareQuery,
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
        Analytics::end_milestone(self, ctx).map(|r| {
            Box::new(PerMilestone {
                at: *ctx.at(),
                inner: r,
            }) as _
        })
    }
}

#[async_trait::async_trait]
trait DailyAnalytics {
    type Measurement;
    async fn handle_date(&mut self, date: time::Date, db: &MongoDb) -> eyre::Result<Self::Measurement>;
}

// This trait allows using the above implementation dynamically
#[async_trait::async_trait]
trait DynDailyAnalytics: Send {
    async fn handle_date(&mut self, date: time::Date, db: &MongoDb) -> eyre::Result<Box<dyn PrepareQuery>>;
}

#[async_trait::async_trait]
impl<T: DailyAnalytics + Send> DynDailyAnalytics for T
where
    PerDay<T::Measurement>: 'static + PrepareQuery,
{
    async fn handle_date(&mut self, date: time::Date, db: &MongoDb) -> eyre::Result<Box<dyn PrepareQuery>> {
        DailyAnalytics::handle_date(self, date, db)
            .await
            .map(|r| Box::new(PerDay { date, inner: r }) as _)
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
            AnalyticsChoice::ActiveAddresses => Box::<AddressActivityAnalytics>::default() as _,
            AnalyticsChoice::LedgerOutputs => Box::new(LedgerOutputMeasurement::init(unspent_outputs)) as _,
            AnalyticsChoice::LedgerSize => {
                Box::new(LedgerSizeAnalytics::init(protocol_params.clone(), unspent_outputs)) as _
            }
            AnalyticsChoice::MilestoneSize => Box::<MilestoneSizeMeasurement>::default() as _,
            AnalyticsChoice::OutputActivity => Box::<OutputActivityMeasurement>::default() as _,
            AnalyticsChoice::ProtocolParameters => Box::<ProtocolParamsMeasurement>::default() as _,
            AnalyticsChoice::TransactionSizeDistribution => Box::<TransactionSizeMeasurement>::default() as _,
            AnalyticsChoice::UnclaimedTokens => Box::new(UnclaimedTokenMeasurement::init(unspent_outputs)) as _,
            AnalyticsChoice::UnlockConditions => Box::new(UnlockConditionMeasurement::init(unspent_outputs)) as _,
        })
    }
}

#[allow(missing_docs)]
pub struct DailyAnalytic(Box<dyn DynDailyAnalytics>);

impl DailyAnalytic {
    /// Init an analytic from a choice and ledger state.
    pub fn init(choice: &DailyAnalyticsChoice) -> Self {
        Self(match choice {
            DailyAnalyticsChoice::ActiveAddresses => Box::<DailyAddressActivityMeasurement>::default() as _,
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

impl MongoDb {
    /// Update a list of daily analytics with this date
    pub async fn update_daily_analytics(
        &self,
        analytics: &mut [DailyAnalytic],
        influxdb: &InfluxDb,
        date: time::Date,
    ) -> eyre::Result<()> {
        for analytic in analytics {
            influxdb
                .insert_measurement(analytic.0.handle_date(date, self).await?)
                .await?;
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

#[derive(Clone, Debug)]
#[allow(missing_docs)]
struct PerDay<M> {
    date: time::Date,
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
