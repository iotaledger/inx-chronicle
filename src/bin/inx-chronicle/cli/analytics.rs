// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use chronicle::{
    analytics::{Analytic, AnalyticsInterval, IntervalAnalytic},
    db::{
        influxdb::{
            config::{all_analytics, all_interval_analytics, IntervalAnalyticsChoice},
            AnalyticsChoice, InfluxDb,
        },
        mongodb::collections::{MilestoneCollection, OutputCollection},
        MongoDb,
    },
    model::{protocol::ProtocolParameters, tangle::MilestoneIndex},
    tangle::{InputSource, Tangle},
};
use clap::Parser;
use futures::TryStreamExt;
use time::{Date, OffsetDateTime};
use tracing::{debug, info};

use crate::config::ChronicleConfig;

/// This command accepts both milestone index and date ranges.
///
/// The following rules apply:
///
/// - If both milestone and date are specified, the date will be used for interval analytics
/// while the milestone will be used for per-milestone analytics.
///
/// - If only the milestone is specified, the date will be inferred from the milestone timestamp.
///
/// - If only the date is specified, the milestone will be inferred from the available data from that date.
///
/// - If neither are specified, then the entire range of available data will be used.
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
pub struct FillAnalyticsCommand {
    /// The inclusive starting milestone index for per-milestone analytics.
    #[arg(short, long)]
    start_milestone: Option<MilestoneIndex>,
    /// The inclusive ending milestone index for per-milestone analytics.
    #[arg(short, long)]
    end_milestone: Option<MilestoneIndex>,
    /// The inclusive starting date (YYYY-MM-DD).
    #[arg(long, value_parser = parse_date)]
    start_date: Option<Date>,
    /// The inclusive ending date (YYYY-MM-DD).
    #[arg(long, value_parser = parse_date)]
    end_date: Option<Date>,
    /// The number of parallel tasks to use when filling per-milestone analytics.
    #[arg(short, long, default_value_t = 1)]
    num_tasks: usize,
    /// Select a subset of per-milestone analytics to compute.
    #[arg(long, value_enum, default_values_t = all_analytics())]
    analytics: Vec<AnalyticsChoice>,
    /// The input source to use for filling per-milestone analytics.
    #[arg(short, long, value_name = "INPUT_SOURCE", default_value = "mongo-db")]
    input_source: InputSourceChoice,
    /// The interval to use for interval analytics.
    #[arg(long, default_value = "day")]
    interval: AnalyticsInterval,
    /// The number of parallel tasks to use when filling interval analytics.
    #[arg(long, default_value_t = 1)]
    num_interval_tasks: usize,
    /// Select a subset of interval analytics to compute.
    #[arg(long, value_enum, default_values_t = all_interval_analytics())]
    interval_analytics: Vec<IntervalAnalyticsChoice>,
}

fn parse_date(s: &str) -> eyre::Result<Date> {
    Ok(Date::parse(
        s,
        time::macros::format_description!("[year]-[month]-[day]"),
    )?)
}

impl FillAnalyticsCommand {
    pub async fn handle(&self, config: &ChronicleConfig) -> eyre::Result<()> {
        let Self {
            start_milestone,
            end_milestone,
            start_date,
            end_date,
            num_tasks,
            analytics,
            input_source,
            interval,
            interval_analytics,
            num_interval_tasks,
        } = self;
        tracing::info!("Connecting to database using hosts: `{}`.", config.mongodb.hosts_str()?);
        let db = MongoDb::connect(&config.mongodb).await?;
        let start_milestone = if let Some(index) = start_milestone {
            let ts = db
                .collection::<MilestoneCollection>()
                .get_milestone_timestamp(*index)
                .await?
                .ok_or_else(|| eyre::eyre!("Could not find requested milestone {}.", index))?;
            index.with_timestamp(ts)
        } else if let Some(start_date) = start_date {
            let ts = start_date.midnight().assume_utc().unix_timestamp();
            db.collection::<MilestoneCollection>()
                .find_first_milestone((ts as u32).into())
                .await?
                .ok_or_else(|| eyre::eyre!("No milestones found after {start_date}."))?
        } else {
            db.collection::<MilestoneCollection>()
                .get_oldest_milestone()
                .await?
                .ok_or_else(|| eyre::eyre!("No milestones in database."))?
        };
        let (start_milestone, start_date) = (
            start_milestone.milestone_index,
            start_date.unwrap_or(
                OffsetDateTime::try_from(start_milestone.milestone_timestamp)
                    .unwrap()
                    .date(),
            ),
        );
        let end_milestone = if let Some(index) = end_milestone {
            let ts = db
                .collection::<MilestoneCollection>()
                .get_milestone_timestamp(*index)
                .await?
                .ok_or_else(|| eyre::eyre!("Could not find requested milestone {}.", index))?;
            index.with_timestamp(ts)
        } else if let Some(end_date) = end_date {
            let ts = end_date.next_day().unwrap().midnight().assume_utc().unix_timestamp();
            db.collection::<MilestoneCollection>()
                .find_last_milestone((ts as u32).into())
                .await?
                .ok_or_else(|| eyre::eyre!("No milestones found before {end_date}."))?
        } else {
            db.collection::<MilestoneCollection>()
                .get_newest_milestone()
                .await?
                .ok_or_else(|| eyre::eyre!("No milestones in database."))?
        };
        let (end_milestone, end_date) = (
            end_milestone.milestone_index,
            end_date.unwrap_or(
                OffsetDateTime::try_from(end_milestone.milestone_timestamp)
                    .unwrap()
                    .date(),
            ),
        );
        if end_milestone < start_milestone {
            eyre::bail!("No milestones in range: {start_milestone}..={end_milestone}.");
        }
        if end_date < start_date {
            eyre::bail!("No dates in range: {start_date}..={end_date}.");
        }
        let influx_db = InfluxDb::connect(&config.influxdb).await?;

        tokio::try_join!(
            async {
                match input_source {
                    #[cfg(feature = "inx")]
                    InputSourceChoice::Inx => {
                        tracing::info!("Connecting to INX at url `{}`.", config.inx.url);
                        let inx = chronicle::inx::Inx::connect(config.inx.url.clone()).await?;
                        fill_analytics(
                            &db,
                            &influx_db,
                            &inx,
                            start_milestone,
                            end_milestone,
                            *num_tasks,
                            analytics,
                        )
                        .await?;
                    }
                    InputSourceChoice::MongoDb => {
                        fill_analytics(
                            &db,
                            &influx_db,
                            &db,
                            start_milestone,
                            end_milestone,
                            *num_tasks,
                            analytics,
                        )
                        .await?;
                    }
                }
                Ok(())
            },
            fill_interval_analytics(
                &db,
                &influx_db,
                start_date,
                end_date,
                *interval,
                *num_interval_tasks,
                interval_analytics
            )
        )?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum InputSourceChoice {
    MongoDb,
    #[cfg(feature = "inx")]
    Inx,
}

pub async fn fill_analytics<I: 'static + InputSource + Clone>(
    db: &MongoDb,
    influx_db: &InfluxDb,
    input_source: &I,
    start_milestone: MilestoneIndex,
    end_milestone: MilestoneIndex,
    num_tasks: usize,
    analytics: &[AnalyticsChoice],
) -> eyre::Result<()> {
    let mut join_set = tokio::task::JoinSet::new();

    let chunk_size = (end_milestone.0 - start_milestone.0) / num_tasks as u32;
    let remainder = (end_milestone.0 - start_milestone.0) % num_tasks as u32;

    let analytics_choices = analytics.iter().copied().collect::<HashSet<_>>();
    info!("Computing the following analytics: {analytics_choices:?}");

    let mut chunk_start_milestone = start_milestone;

    for i in 0..num_tasks {
        let db = db.clone();
        let influx_db = influx_db.clone();
        let tangle = Tangle::from(input_source.clone());
        let analytics_choices = analytics_choices.clone();

        let actual_chunk_size = chunk_size + (i < remainder as usize) as u32;
        debug!(
            "Task {i} chunk {chunk_start_milestone}..{}, {actual_chunk_size} milestones",
            chunk_start_milestone + actual_chunk_size,
        );

        join_set.spawn(async move {
            let mut state: Option<AnalyticsState> = None;

            let mut milestone_stream = tangle
                .milestone_stream(chunk_start_milestone..chunk_start_milestone + actual_chunk_size)
                .await?;

            loop {
                let start_time = std::time::Instant::now();

                if let Some(milestone) = milestone_stream.try_next().await? {
                    // Check if the protocol params changed (or we just started)
                    if !matches!(&state, Some(state) if state.prev_protocol_params == milestone.protocol_params) {
                        // Only get the ledger state for milestones after the genesis since it requires
                        // getting the previous milestone data.
                        let ledger_state = if milestone.at.milestone_index.0 > 0 {
                            db.collection::<OutputCollection>()
                                .get_unspent_output_stream(milestone.at.milestone_index - 1)
                                .await?
                                .try_collect::<Vec<_>>()
                                .await?
                        } else {
                            panic!("There should be no milestone with index 0.");
                        };

                        let analytics = analytics_choices
                            .iter()
                            .map(|choice| {
                                Analytic::init(
                                    choice,
                                    &milestone.protocol_params,
                                    &ledger_state,
                                    milestone.at.milestone_timestamp,
                                )
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

                    let elapsed = start_time.elapsed();
                    #[cfg(feature = "metrics")]
                    {
                        influx_db
                            .metrics()
                            .insert(chronicle::metrics::AnalyticsMetrics {
                                time: chrono::Utc::now(),
                                milestone_index: milestone.at.milestone_index,
                                analytics_time: elapsed.as_millis() as u64,
                                chronicle_version: std::env!("CARGO_PKG_VERSION").to_string(),
                            })
                            .await?;
                    }
                    info!(
                        "Task {i} finished analytics for milestone {} in {}ms.",
                        milestone.at.milestone_index,
                        elapsed.as_millis()
                    );
                } else {
                    break;
                }
            }
            eyre::Result::<_>::Ok(())
        });

        chunk_start_milestone += actual_chunk_size;
    }
    while let Some(res) = join_set.join_next().await {
        // Panic: Acceptable risk
        res.unwrap()?;
    }
    Ok(())
}

pub async fn fill_interval_analytics(
    db: &MongoDb,
    influx_db: &InfluxDb,
    start_date: Date,
    end_date: Date,
    interval: AnalyticsInterval,
    num_tasks: usize,
    analytics: &[IntervalAnalyticsChoice],
) -> eyre::Result<()> {
    let mut join_set = tokio::task::JoinSet::new();

    let analytics_choices = analytics.iter().copied().collect::<HashSet<_>>();
    info!("Computing the following {interval} analytics for {start_date}..{end_date}: {analytics_choices:?}",);

    for i in 0..num_tasks {
        let db = db.clone();
        let influx_db = influx_db.clone();
        let analytics_choices = analytics_choices.clone();
        let mut date = start_date;
        for _ in 0..i {
            date = interval.end_date(&date);
            if date >= end_date {
                break;
            }
        }

        let mut analytics = analytics_choices.iter().map(IntervalAnalytic::init).collect::<Vec<_>>();

        join_set.spawn(async move {
            while date < end_date {
                let start_time = std::time::Instant::now();

                db.update_interval_analytics(&mut analytics, &influx_db, date, interval)
                    .await?;

                let elapsed = start_time.elapsed().as_millis();
                info!(
                    "Task {i} finished {interval} analytics for {date}..{} in {elapsed}ms.",
                    interval.end_date(&date)
                );
                for _ in 0..num_tasks {
                    date = interval.end_date(&date);
                    if date >= end_date {
                        break;
                    }
                }
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
    pub analytics: Vec<Analytic>,
    pub prev_protocol_params: ProtocolParameters,
}
