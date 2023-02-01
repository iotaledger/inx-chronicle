// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Various analytics that give insight into the usage of the tangle.

use futures::TryStreamExt;
use thiserror::Error;
use time::OffsetDateTime;

use self::{
    ledger::{
        AddressActivity, AddressBalanceAnalytics, AliasActivityAnalytics, BaseTokenActivityAnalytics,
        LedgerOutputAnalytics, LedgerSizeAnalytics, NftActivityAnalytics, TransactionAnalytics,
        UnclaimedTokenAnalytics, UnlockConditionAnalytics,
    },
    tangle::{BlockActivityAnalytics, BlockAnalytics, MilestoneSizeAnalytics},
};
use crate::{
    db::{
        collections::analytics::{Measurement, OutputActivityAnalyticsResult, PerMilestone, TimeInterval},
        influxdb::{AnalyticsChoice, InfluxDb},
    },
    tangle::{BlockData, InputSource, LedgerUpdateStore, Milestone},
    types::{
        ledger::LedgerOutput,
        stardust::block::{payload::TransactionEssence, Input, Payload},
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
    LedgerSize(LedgerSizeAnalytics),
    MilestoneSize(MilestoneSizeAnalytics),
    OutputActivity {
        nft: NftActivityAnalytics,
        alias: AliasActivityAnalytics,
    },
    ProtocolParameters(ProtocolParameters),
    UnclaimedTokens(UnclaimedTokenAnalytics),
    UnlockConditions(UnlockConditionAnalytics),
}

impl Analytic {
    /// Init an analytic from a choice and ledger state.
    pub fn init<'a>(
        choice: &AnalyticsChoice,
        protocol_params: &ProtocolParameters,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
    ) -> Self {
        match choice {
            AnalyticsChoice::AddressBalance => Analytic::AddressBalance(AddressBalanceAnalytics::init(unspent_outputs)),
            AnalyticsChoice::BaseTokenActivity => Analytic::BaseTokenActivity(Default::default()),
            AnalyticsChoice::BlockActivity => Analytic::BlockActivity(Default::default()),
            AnalyticsChoice::DailyActiveAddresses => Analytic::DailyActiveAddresses(AddressActivity::init(
                OffsetDateTime::now_utc().date().midnight().assume_utc(),
                time::Duration::days(1),
                unspent_outputs,
            )),
            AnalyticsChoice::LedgerOutputs => Analytic::LedgerOutputs(LedgerOutputAnalytics::init(unspent_outputs)),
            AnalyticsChoice::LedgerSize => {
                Analytic::LedgerSize(LedgerSizeAnalytics::init(protocol_params.clone(), unspent_outputs))
            }
            AnalyticsChoice::OutputActivity => Analytic::OutputActivity {
                nft: Default::default(),
                alias: Default::default(),
            },
            AnalyticsChoice::ProtocolParameters => Analytic::ProtocolParameters(protocol_params.clone()),
            AnalyticsChoice::UnclaimedTokens => {
                Analytic::UnclaimedTokens(UnclaimedTokenAnalytics::init(unspent_outputs))
            }
            AnalyticsChoice::UnlockConditions => {
                Analytic::UnlockConditions(UnlockConditionAnalytics::init(unspent_outputs))
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
        let ledger_updates = self.ledger_updates().await?;

        self.begin_milestone(analytics);

        while let Some(block_data) = cone_stream.try_next().await? {
            self.handle_block(analytics, &block_data, &ledger_updates)?;
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
                Analytic::OutputActivity { nft, alias } => {
                    nft.begin_milestone(self.at);
                    alias.begin_milestone(self.at);
                }
                Analytic::ProtocolParameters(_) => (),
                Analytic::UnclaimedTokens(stat) => stat.begin_milestone(self.at),
                Analytic::UnlockConditions(stat) => stat.begin_milestone(self.at),
            }
        }
    }

    fn handle_block(
        &self,
        analytics: &mut [Analytic],
        block_data: &BlockData,
        ledger_updates: &LedgerUpdateStore,
    ) -> eyre::Result<()> {
        if let Some(Payload::Transaction(payload)) = &block_data.block.payload {
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
                    Ok(ledger_updates
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
                    Analytic::OutputActivity { nft, alias } => {
                        nft.handle_transaction(&consumed, &created);
                        alias.handle_transaction(&consumed, &created);
                    }
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
            Analytic::OutputActivity { nft, alias } => {
                let nft = nft.end_milestone(self.at);
                let alias = alias.end_milestone(self.at);
                if nft.is_some() || alias.is_some() {
                    Some(Measurement::OutputActivity(PerMilestone {
                        at: self.at,
                        inner: OutputActivityAnalyticsResult {
                            alias: alias.unwrap_or_default(),
                            nft: nft.unwrap_or_default(),
                        },
                    }))
                } else {
                    None
                }
            }
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
