// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{
    analytics::{Analytic, AnalyticsInterval, IntervalAnalytic},
    db::{
        influxdb::{config::IntervalAnalyticsChoice, AnalyticsChoice, InfluxDb},
        MongoDb,
    },
    tangle::{InputSource, Tangle},
    types::tangle::MilestoneIndex,
};
use futures::TryStreamExt;
use tracing::{debug, info};

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

    let analytics_choices = if analytics.is_empty() {
        super::influxdb::all_analytics()
    } else {
        analytics.iter().copied().collect()
    };
    info!("Computing the following analytics: {:?}", analytics_choices);

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

            let (buffer_in, mut buffer) = tokio::sync::mpsc::channel(10);

            tokio::try_join!(
                async {
                    while let Some(milestone) = milestone_stream.try_next().await? {
                        buffer_in.send(milestone).await.map_err(|e| {
                            eyre::eyre!("failed to send milestone {} across channel", e.0.at.milestone_index)
                        })?;
                    }
                    eyre::Result::<_>::Ok(())
                },
                async {
                    loop {
                        let start_time = std::time::Instant::now();
                        if let Some(milestone) = buffer.recv().await {
                            // Check if the protocol params changed (or we just started)
                            if !matches!(&state, Some(state) if state.prev_protocol_params == milestone.protocol_params)
                            {
                                debug!("Getting ledger state at {}", milestone.at.milestone_index - 1);
                                // Only get the ledger state for milestones after the genesis since it requires
                                // getting the previous milestone data.
                                let ledger_state = if milestone.at.milestone_index.0 > 0 {
                                    db.collection::<chronicle::db::collections::OutputCollection>()
                                        .get_unspent_output_stream(milestone.at.milestone_index - 1)
                                        .await?
                                        .try_collect::<Vec<_>>()
                                        .await?
                                } else {
                                    panic!("There should be no milestone with index 0.");
                                };

                                let analytics = analytics_choices
                                    .iter()
                                    .map(|choice| Analytic::init(choice, &milestone.protocol_params, &ledger_state))
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
                                    .insert(chronicle::db::collections::metrics::AnalyticsMetrics {
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

                    Ok(())
                }
            )?;

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
    start_date: time::Date,
    end_date: time::Date,
    interval: AnalyticsInterval,
    num_tasks: usize,
    analytics: &[IntervalAnalyticsChoice],
) -> eyre::Result<()> {
    let mut join_set = tokio::task::JoinSet::new();

    let analytics_choices = if analytics.is_empty() {
        super::influxdb::all_interval_analytics()
    } else {
        analytics.iter().copied().collect()
    };
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
    pub analytics: Vec<chronicle::analytics::Analytic>,
    pub prev_protocol_params: chronicle::types::tangle::ProtocolParameters,
}
