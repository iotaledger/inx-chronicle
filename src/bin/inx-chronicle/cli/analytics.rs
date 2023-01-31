// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{
    analytics::{
        ledger::{
            AddressActivity, AddressBalanceAnalytics, LedgerOutputAnalytics, LedgerSizeAnalytics,
            UnclaimedTokenAnalytics,
        },
        Analytic,
    },
    db::{
        influxdb::{AnalyticsChoice, InfluxDb},
        MongoDb,
    },
    tangle::Tangle,
    types::tangle::MilestoneIndex,
};
use futures::TryStreamExt;
use time::OffsetDateTime;

pub async fn fill_analytics(
    db: &MongoDb,
    influx_db: &InfluxDb,
    start_milestone: MilestoneIndex,
    end_milestone: MilestoneIndex,
    num_tasks: usize,
    analytics: &[AnalyticsChoice],
) -> eyre::Result<()> {
    let mut join_set = tokio::task::JoinSet::new();
    let chunk_size = (end_milestone.0 - start_milestone.0) / num_tasks as u32
        + ((end_milestone.0 - start_milestone.0) % num_tasks as u32 != 0) as u32;
    for i in 0..num_tasks {
        let db = db.clone();
        let influx_db = influx_db.clone();
        let analytics_choices = if analytics.is_empty() {
            super::influxdb::all_analytics()
        } else {
            analytics.iter().copied().collect()
        };

        join_set.spawn(async move {
            tracing::info!("Computing the following analytics: {:?}", analytics_choices);

            let start_milestone = start_milestone + i as u32 * chunk_size;

            let mut state: Option<AnalyticsState> = None;

            let tangle = Tangle::from_mongodb(&db);
            let mut milestone_stream = tangle
                .milestone_stream(start_milestone..start_milestone + chunk_size)
                .await?;
            while let Some(milestone) = milestone_stream.try_next().await? {
                #[cfg(feature = "metrics")]
                let start_time = std::time::Instant::now();

                // Check if the protocol params changed (or we just started)
                if !matches!(&state, Some(state) if state.prev_protocol_params == milestone.protocol_params) {
                    let ledger_state = db
                        .collection::<chronicle::db::collections::OutputCollection>()
                        .get_unspent_output_stream(milestone.at.milestone_index)
                        .await?
                        .try_collect::<Vec<_>>()
                        .await?;

                    let analytics = analytics_choices
                        .iter()
                        .map(|choice| match choice {
                            AnalyticsChoice::AddressBalance => {
                                Analytic::AddressBalance(AddressBalanceAnalytics::init(&ledger_state))
                            }
                            AnalyticsChoice::BaseTokenActivity => Analytic::BaseTokenActivity(Default::default()),
                            AnalyticsChoice::BlockActivity => Analytic::BlockActivity(Default::default()),
                            AnalyticsChoice::DailyActiveAddresses => {
                                Analytic::DailyActiveAddresses(AddressActivity::init(
                                    OffsetDateTime::now_utc().date().midnight().assume_utc(),
                                    time::Duration::days(1),
                                    &ledger_state,
                                ))
                            }
                            AnalyticsChoice::LedgerOutputs => {
                                Analytic::LedgerOutputs(LedgerOutputAnalytics::init(&ledger_state))
                            }
                            AnalyticsChoice::LedgerSize => Analytic::LedgerSize(LedgerSizeAnalytics::init(
                                milestone.protocol_params.clone(),
                                &ledger_state,
                            )),
                            AnalyticsChoice::OutputActivity => Analytic::OutputActivity {
                                nft: Default::default(),
                                alias: Default::default(),
                            },
                            AnalyticsChoice::ProtocolParameters => {
                                Analytic::ProtocolParameters(milestone.protocol_params.clone())
                            }
                            AnalyticsChoice::UnclaimedTokens => {
                                Analytic::UnclaimedTokens(UnclaimedTokenAnalytics::init(&ledger_state))
                            }
                            AnalyticsChoice::UnlockConditions => {
                                Analytic::UnlockConditions(UnlockConditionAnalytics::init(&ledger_state))
                            }
                        })
                        .collect::<Vec<_>>();
                    state = Some(AnalyticsState {
                        analytics,
                        prev_protocol_params: milestone.protocol_params.clone(),
                    });
                }

                // Unwrap: safe because we guarantee it is initialized above
                milestone
                    .update_analytics(&mut state.as_mut().unwrap().analytics, &influx_db)
                    .await?;

                #[cfg(feature = "metrics")]
                {
                    let elapsed = start_time.elapsed();
                    influx_db
                        .metrics()
                        .insert(chronicle::db::collections::metrics::AnalyticsMetrics {
                            time: chrono::Utc::now(),
                            milestone_index: milestone.at.milestone_index,
                            analytics_time: elapsed.as_millis() as u64,
                            chronicle_version: std::env!("CARGO_PKG_VERSION").to_string(),
                        })
                        .await?;
                }
                tracing::info!("Finished analytics for milestone {}", milestone.at.milestone_index);
            }
            eyre::Result::<_>::Ok(())
        });
    }
    while let Some(res) = join_set.join_next().await {
        // Panic: Acceptable risk
        res.unwrap()?;
    }
    Ok(())
}

pub struct AnalyticsState {
    pub analytics: Vec<chronicle::analytics::Analytic>,
    pub prev_protocol_params: chronicle::types::tangle::ProtocolParameters,
}
