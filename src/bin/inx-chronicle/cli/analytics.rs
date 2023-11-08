// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use chronicle::{
    analytics::{Analytic, AnalyticsContext, AnalyticsInterval, IntervalAnalytic},
    db::{
        influxdb::{
            config::{all_analytics, all_interval_analytics, IntervalAnalyticsChoice},
            AnalyticsChoice, InfluxDb,
        },
        mongodb::collections::{OutputCollection, ProtocolUpdateCollection},
        MongoDb,
    },
    tangle::{InputSource, Tangle},
};
use clap::Parser;
use futures::TryStreamExt;
use iota_sdk::types::block::{protocol::ProtocolParameters, slot::SlotIndex};
use time::{Date, OffsetDateTime};
use tracing::{debug, info};

use crate::config::ChronicleConfig;

/// This command accepts both slot index and date ranges. The following rules apply:
///
/// - If both slot and date are specified, the date will be used for interval analytics
/// while the slot will be used for per-slot analytics.
///
/// - If only the slot is specified, the date will be inferred from the slot timestamp.
///
/// - If only the date is specified, the slot will be inferred from the available data from that date.
///
/// - If neither are specified, then the entire range of available data will be used.
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
pub struct FillAnalyticsCommand {
    /// The inclusive starting slot index for per-slot analytics.
    #[arg(short, long)]
    start_index: Option<SlotIndex>,
    /// The inclusive ending slot index for per-slot analytics.
    #[arg(short, long)]
    end_index: Option<SlotIndex>,
    /// The inclusive starting date (YYYY-MM-DD).
    #[arg(long, value_parser = parse_date)]
    start_date: Option<Date>,
    /// The inclusive ending date (YYYY-MM-DD).
    #[arg(long, value_parser = parse_date)]
    end_date: Option<Date>,
    /// The number of parallel tasks to use when filling per-slot analytics.
    #[arg(short, long, default_value_t = 1)]
    num_tasks: usize,
    /// Select a subset of per-slot analytics to compute.
    #[arg(long, value_enum, default_values_t = all_analytics())]
    analytics: Vec<AnalyticsChoice>,
    /// The input source to use for filling per-slot analytics.
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
            start_index,
            end_index,
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
        let protocol_params = db
            .collection::<ProtocolUpdateCollection>()
            .get_latest_protocol_parameters()
            .await?
            .ok_or_else(|| eyre::eyre!("No protocol parameters in database."))?
            .parameters;
        let start_index = if let Some(index) = start_index {
            *index
        } else if let Some(start_date) = start_date {
            let ts = start_date.midnight().assume_utc().unix_timestamp_nanos() as u64;
            SlotIndex::from_timestamp(
                ts,
                protocol_params.genesis_unix_timestamp(),
                protocol_params.slot_duration_in_seconds(),
            )
        } else {
            todo!("get the oldest slot in the DB")
        };
        let (start_index, start_date) = (
            start_index,
            start_date.unwrap_or(
                OffsetDateTime::from_unix_timestamp_nanos(start_index.to_timestamp(
                    protocol_params.genesis_unix_timestamp(),
                    protocol_params.slot_duration_in_seconds(),
                ) as _)
                .unwrap()
                .date(),
            ),
        );
        let end_index = if let Some(index) = end_index {
            *index
        } else if let Some(end_date) = end_date {
            let ts = end_date
                .next_day()
                .unwrap()
                .midnight()
                .assume_utc()
                .unix_timestamp_nanos() as u64;
            SlotIndex::from_timestamp(
                ts,
                protocol_params.genesis_unix_timestamp(),
                protocol_params.slot_duration_in_seconds(),
            )
        } else {
            todo!("get the newest slot in the DB")
        };
        let (end_index, end_date) = (
            end_index,
            end_date.unwrap_or(
                OffsetDateTime::from_unix_timestamp_nanos(end_index.to_timestamp(
                    protocol_params.genesis_unix_timestamp(),
                    protocol_params.slot_duration_in_seconds(),
                ) as _)
                .unwrap()
                .date(),
            ),
        );
        if end_index < start_index {
            eyre::bail!("No slots in range: {start_index}..={end_index}.");
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
                        let inx = chronicle::inx::Inx::connect(&config.inx.url).await?;
                        fill_analytics(&db, &influx_db, &inx, start_index, end_index, *num_tasks, analytics).await?;
                    }
                    InputSourceChoice::MongoDb => {
                        fill_analytics(&db, &influx_db, &db, start_index, end_index, *num_tasks, analytics).await?;
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
    start_index: SlotIndex,
    end_index: SlotIndex,
    num_tasks: usize,
    analytics: &[AnalyticsChoice],
) -> eyre::Result<()> {
    let mut join_set = tokio::task::JoinSet::new();

    let chunk_size = (end_index.0 - start_index.0) / num_tasks as u32;
    let remainder = (end_index.0 - start_index.0) % num_tasks as u32;

    let analytics_choices = analytics.iter().copied().collect::<HashSet<_>>();
    info!("Computing the following analytics: {analytics_choices:?}");

    let mut chunk_start_slot = start_index;

    for i in 0..num_tasks {
        let db = db.clone();
        let influx_db = influx_db.clone();
        let tangle = Tangle::from(input_source.clone());
        let analytics_choices = analytics_choices.clone();

        let actual_chunk_size = chunk_size + (i < remainder as usize) as u32;
        debug!(
            "Task {i} chunk {chunk_start_slot}..{}, {actual_chunk_size} slots",
            chunk_start_slot + actual_chunk_size,
        );

        join_set.spawn(async move {
            let mut state: Option<AnalyticsState> = None;

            let mut slot_stream = tangle
                .slot_stream(chunk_start_slot..chunk_start_slot + actual_chunk_size)
                .await?;

            loop {
                let start_time = std::time::Instant::now();

                if let Some(slot) = slot_stream.try_next().await? {
                    // Check if the protocol params changed (or we just started)
                    if !matches!(&state, Some(state) if state.prev_protocol_params == slot.protocol_params.parameters) {
                        // Only get the ledger state for slots after the genesis since it requires
                        // getting the previous slot data.
                        let ledger_state = if slot.slot_index().0 > 0 {
                            db.collection::<OutputCollection>()
                                .get_unspent_output_stream(slot.slot_index() - 1)
                                .await?
                                .try_collect::<Vec<_>>()
                                .await?
                        } else {
                            panic!("There should be no slots with index 0.");
                        };

                        let analytics = analytics_choices
                            .iter()
                            .map(|choice| Analytic::init(choice, &slot.protocol_params.parameters, &ledger_state))
                            .collect::<Vec<_>>();
                        state = Some(AnalyticsState {
                            analytics,
                            prev_protocol_params: slot.protocol_params.parameters.clone(),
                        });
                    }

                    // Unwrap: safe because we guarantee it is initialized above
                    slot.update_analytics(&mut state.as_mut().unwrap().analytics, &influx_db)
                        .await?;

                    let elapsed = start_time.elapsed();
                    #[cfg(feature = "metrics")]
                    {
                        influx_db
                            .metrics()
                            .insert(chronicle::metrics::AnalyticsMetrics {
                                time: chrono::Utc::now(),
                                slot_index: slot.slot_index().0,
                                analytics_time: elapsed.as_millis() as u64,
                                chronicle_version: std::env!("CARGO_PKG_VERSION").to_string(),
                            })
                            .await?;
                    }
                    info!(
                        "Task {i} finished analytics for slot {} in {}ms.",
                        slot.slot_index(),
                        elapsed.as_millis()
                    );
                } else {
                    break;
                }
            }
            eyre::Result::<_>::Ok(())
        });

        chunk_start_slot += actual_chunk_size;
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
