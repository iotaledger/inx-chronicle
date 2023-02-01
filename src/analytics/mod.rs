// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Various analytics that give insight into the usage of the tangle.

use futures::TryStreamExt;
use thiserror::Error;
use time::OffsetDateTime;

use self::{
    influx::{Measurement, PerMilestone, TimeInterval},
    ledger::{
        AddressActivityAnalytics, AddressBalancesAnalytics, BaseTokenActivityMeasurement, LedgerOutputMeasurement,
        LedgerSizeAnalytics, OutputActivityMeasurement, TransactionAnalytics, UnclaimedTokenMeasurement,
        UnlockConditionMeasurement,
    },
    tangle::{BlockActivityMeasurement, BlockAnalytics, MilestoneSizeMeasurement},
};
use crate::{
    db::influxdb::{AnalyticsChoice, InfluxDb},
    tangle::{BlockData, InputSource, Milestone},
    types::{
        ledger::LedgerOutput,
        stardust::block::{payload::TransactionEssence, Input, Payload},
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

pub mod influx;
pub mod ledger;
pub mod tangle;

#[allow(missing_docs)]
pub enum Analytic {
    AddressBalance(AddressBalancesAnalytics),
    BaseTokenActivity(BaseTokenActivityMeasurement),
    BlockActivity(BlockActivityMeasurement),
    DailyActiveAddresses(AddressActivityAnalytics),
    LedgerOutputs(LedgerOutputMeasurement),
    LedgerSize(LedgerSizeAnalytics),
    MilestoneSize(MilestoneSizeMeasurement),
    OutputActivity(OutputActivityMeasurement),
    ProtocolParameters(ProtocolParameters),
    UnclaimedTokens(UnclaimedTokenMeasurement),
    UnlockConditions(UnlockConditionMeasurement),
}

impl Analytic {
    /// Init an analytic from a choice and ledger state.
    pub fn init<'a>(
        choice: &AnalyticsChoice,
        protocol_params: &ProtocolParameters,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
    ) -> Self {
        match choice {
            AnalyticsChoice::AddressBalance => {
                Analytic::AddressBalance(AddressBalancesAnalytics::init(unspent_outputs))
            }
            AnalyticsChoice::BaseTokenActivity => Analytic::BaseTokenActivity(Default::default()),
            AnalyticsChoice::BlockActivity => Analytic::BlockActivity(Default::default()),
            AnalyticsChoice::DailyActiveAddresses => Analytic::DailyActiveAddresses(AddressActivityAnalytics::init(
                OffsetDateTime::now_utc().date().midnight().assume_utc(),
                time::Duration::days(1),
                unspent_outputs,
            )),
            AnalyticsChoice::LedgerOutputs => Analytic::LedgerOutputs(LedgerOutputMeasurement::init(unspent_outputs)),
            AnalyticsChoice::LedgerSize => {
                Analytic::LedgerSize(LedgerSizeAnalytics::init(protocol_params.clone(), unspent_outputs))
            }
            AnalyticsChoice::OutputActivity => Analytic::OutputActivity(Default::default()),
            AnalyticsChoice::ProtocolParameters => Analytic::ProtocolParameters(protocol_params.clone()),
            AnalyticsChoice::UnclaimedTokens => {
                Analytic::UnclaimedTokens(UnclaimedTokenMeasurement::init(unspent_outputs))
            }
            AnalyticsChoice::UnlockConditions => {
                Analytic::UnlockConditions(UnlockConditionMeasurement::init(unspent_outputs))
            }
        }
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
            match analytic {
                Analytic::AddressBalance(stat) => stat.begin_milestone(self.at),
                Analytic::BaseTokenActivity(stat) => stat.begin_milestone(self.at),
                Analytic::BlockActivity(stat) => stat.begin_milestone(self.at.milestone_index),
                Analytic::DailyActiveAddresses(stat) => stat.begin_milestone(self.at),
                Analytic::LedgerOutputs(stat) => stat.begin_milestone(self.at),
                Analytic::LedgerSize(stat) => stat.begin_milestone(self.at),
                Analytic::MilestoneSize(stat) => stat.begin_milestone(self.at.milestone_index),
                Analytic::OutputActivity(stat) => stat.begin_milestone(self.at),
                Analytic::ProtocolParameters(_) => (),
                Analytic::UnclaimedTokens(stat) => stat.begin_milestone(self.at),
                Analytic::UnlockConditions(stat) => stat.begin_milestone(self.at),
            }
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
                match analytic {
                    Analytic::AddressBalance(stat) => stat.handle_transaction(&consumed, &created),
                    Analytic::BaseTokenActivity(stat) => stat.handle_transaction(&consumed, &created),
                    Analytic::DailyActiveAddresses(stat) => stat.handle_transaction(&consumed, &created),
                    Analytic::LedgerOutputs(stat) => stat.handle_transaction(&consumed, &created),
                    Analytic::LedgerSize(stat) => stat.handle_transaction(&consumed, &created),
                    Analytic::OutputActivity(stat) => stat.handle_transaction(&consumed, &created),
                    Analytic::UnclaimedTokens(stat) => stat.handle_transaction(&consumed, &created),
                    Analytic::UnlockConditions(stat) => stat.handle_transaction(&consumed, &created),
                    _ => (),
                }
            }
        }
        for analytic in analytics.iter_mut() {
            match analytic {
                Analytic::BlockActivity(stat) => stat.handle_block(block_data),
                Analytic::MilestoneSize(stat) => stat.handle_block(block_data),
                _ => (),
            }
        }
        Ok(())
    }

    async fn end_milestone(&self, analytics: &mut [Analytic], influxdb: &InfluxDb) -> eyre::Result<()> {
        for measurement in analytics.iter_mut().filter_map(|analytic| match analytic {
            Analytic::AddressBalance(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::AddressBalance(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::BaseTokenActivity(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::BaseTokenActivity(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::BlockActivity(stat) => stat.end_milestone(self.at.milestone_index).map(|measurement| {
                Measurement::BlockActivity(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::DailyActiveAddresses(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::DailyActiveAddresses(TimeInterval {
                    from: stat.start_time,
                    to_exclusive: stat.start_time + stat.interval,
                    inner: measurement,
                })
            }),
            Analytic::LedgerOutputs(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::LedgerOutputs(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::LedgerSize(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::LedgerSize(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::MilestoneSize(stat) => stat.end_milestone(self.at.milestone_index).map(|measurement| {
                Measurement::MilestoneSize(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::OutputActivity(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::OutputActivity(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::ProtocolParameters(params) => {
                if params != &self.protocol_params {
                    *params = self.protocol_params.clone();
                    Some(Measurement::ProtocolParameters(PerMilestone {
                        at: self.at,
                        inner: params.clone(),
                    }))
                } else {
                    None
                }
            }
            Analytic::UnclaimedTokens(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::UnclaimedTokens(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::UnlockConditions(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::UnlockConditions(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
        }) {
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
