// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Various analytics that give insight into the usage of the tangle.

use futures::TryStreamExt;
use thiserror::Error;

use self::{
    ledger::{
        AddressActivity, AddressBalanceAnalytics, BaseTokenActivityAnalytics, LedgerOutputAnalytics,
        TransactionAnalytics, UnclaimedTokenAnalytics,
    },
    tangle::{BlockActivityAnalytics, BlockAnalytics},
};
use crate::{
    db::{
        collections::analytics::{Measurement, PerMilestone, TimeInterval},
        influxdb::InfluxDb,
    },
    tangle::{BlockData, InputSource, LedgerUpdateStore, Milestone},
    types::{
        ledger::BlockMetadata,
        stardust::block::{payload::TransactionEssence, Block, Input, Payload},
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

pub mod ledger;
pub mod tangle;

#[allow(missing_docs)]
pub enum Analytic {
    AddressBalance(AddressBalanceAnalytics),
    BaseTokenActivity(BaseTokenActivityAnalytics),
    BlockActivity(BlockActivityAnalytics),
    DailyActiveAddresses(AddressActivity),
    LedgerOutputs(LedgerOutputAnalytics),
    LedgerSize,
    OutputActivity,
    ProtocolParameters(ProtocolParameters),
    UnclaimedTokens(UnclaimedTokenAnalytics),
    UnlockConditions,
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
        let ledger_updates = self.ledger_updates().await?;

        self.begin_milestone(analytics);

        while let Some(BlockData { block, metadata, .. }) = cone_stream.try_next().await? {
            self.handle_block(analytics, &block, &metadata, &ledger_updates)?;
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
                Analytic::LedgerSize => todo!(),
                Analytic::OutputActivity => todo!(),
                Analytic::ProtocolParameters(params) => {
                    if params != &self.protocol_params {
                        *params = self.protocol_params.clone();
                        // TODO: either signal that we should write this at the end or do it now
                        // TODO: re-init all of the analytics, what is this going to take? Can we even do it here?
                    }
                }
                Analytic::UnclaimedTokens(stat) => stat.begin_milestone(self.at),
                Analytic::UnlockConditions => todo!(),
            }
        }
    }

    fn handle_block(
        &self,
        analytics: &mut [Analytic],
        block: &Block,
        block_metadata: &BlockMetadata,
        ledger_updates: &LedgerUpdateStore,
    ) -> eyre::Result<()> {
        match &block.payload {
            Some(Payload::Transaction(payload)) => {
                let TransactionEssence::Regular { inputs, outputs, .. } = &payload.essence;
                let consumed = inputs
                    .iter()
                    .filter_map(|input| match input {
                        Input::Utxo(output_id) => Some(output_id),
                        _ => None,
                    })
                    .map(|output_id| {
                        Ok(ledger_updates
                            .get_consumed(output_id)
                            .ok_or(AnalyticsError::MissingLedgerOutput {
                                output_id: output_id.to_hex(),
                                milestone_index: block_metadata.referenced_by_milestone_index,
                            })?
                            .clone())
                    })
                    .collect::<eyre::Result<Vec<_>>>()?;
                let created = outputs
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        let output_id = (payload.transaction_id, index as _).into();
                        Ok(ledger_updates
                            .get_created(&output_id)
                            .ok_or(AnalyticsError::MissingLedgerOutput {
                                output_id: output_id.to_hex(),
                                milestone_index: block_metadata.referenced_by_milestone_index,
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
                        Analytic::LedgerSize => todo!(),
                        Analytic::OutputActivity => todo!(),
                        Analytic::UnclaimedTokens(stat) => stat.handle_transaction(&consumed, &created),
                        Analytic::UnlockConditions => todo!(),
                        _ => (),
                    }
                }
            }
            _ => (),
        }
        for analytic in analytics.iter_mut() {
            match analytic {
                Analytic::BlockActivity(stat) => stat.handle_block(block, block_metadata),
                _ => (),
            }
        }
        Ok(())
    }

    async fn end_milestone(&self, analytics: &mut [Analytic], influxdb: &InfluxDb) -> eyre::Result<()> {
        for measurement in analytics.iter_mut().filter_map(|analytic| match analytic {
            Analytic::AddressBalance(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::AddressActivity(PerMilestone {
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
            Analytic::LedgerSize => todo!(),
            Analytic::OutputActivity => todo!(),
            Analytic::ProtocolParameters(_params) => {
                todo!();
            }
            Analytic::UnclaimedTokens(stat) => stat.end_milestone(self.at).map(|measurement| {
                Measurement::UnclaimedTokens(PerMilestone {
                    at: self.at,
                    inner: measurement,
                })
            }),
            Analytic::UnlockConditions => todo!(),
        }) {
            influxdb.insert_measurement(measurement).await?;
        }
        Ok(())
    }
}
