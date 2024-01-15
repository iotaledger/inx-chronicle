// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use chronicle::{
    analytics::Analytic,
    db::{
        influxdb::{AnalyticsChoice, InfluxDb},
        mongodb::collections::{ApplicationStateCollection, OutputCollection},
        MongoDb,
    },
    inx::Inx,
    model::tangle::MilestoneIndex,
    tangle::Milestone,
};
use futures::TryStreamExt;

use super::InxWorkerError;
use crate::{cli::analytics::AnalyticsState, inx::InxWorker};

pub struct AnalyticsInfo {
    analytics_choices: HashSet<AnalyticsChoice>,
    state: Option<AnalyticsState>,
    pub synced_index: MilestoneIndex,
}

impl AnalyticsInfo {
    pub async fn init(db: &MongoDb, influx_db: Option<&InfluxDb>) -> eyre::Result<Option<Self>> {
        Ok(if let Some(influx_db) = influx_db {
            Some(Self {
                analytics_choices: if influx_db.config().analytics.is_empty() {
                    chronicle::db::influxdb::config::all_analytics()
                } else {
                    influx_db.config().analytics.iter().copied().collect()
                },
                state: None,
                synced_index: db
                    .collection::<ApplicationStateCollection>()
                    .get_starting_index()
                    .await?
                    .ok_or(InxWorkerError::MissingAppState)?
                    .milestone_index,
            })
        } else {
            None
        })
    }
}

impl InxWorker {
    pub async fn update_analytics<'a>(
        &self,
        milestone: &Milestone<'a, Inx>,
        AnalyticsInfo {
            analytics_choices,
            state,
            ..
        }: &mut AnalyticsInfo,
    ) -> eyre::Result<()> {
        if let (Some(influx_db), analytics_choices) = (&self.influx_db, analytics_choices) {
            if influx_db.config().analytics_enabled {
                // Check if the protocol params changed (or we just started)
                if !matches!(&state, Some(state) if state.prev_protocol_params == milestone.protocol_params) {
                    let ledger_state = self
                        .db
                        .collection::<OutputCollection>()
                        .get_unspent_output_stream(milestone.at.milestone_index - 1)
                        .await?
                        .try_collect::<Vec<_>>()
                        .await?;

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
                    *state = Some(AnalyticsState {
                        analytics,
                        prev_protocol_params: milestone.protocol_params.clone(),
                    });
                }

                // Unwrap: safe because we guarantee it is initialized above
                milestone
                    .update_analytics(&mut state.as_mut().unwrap().analytics, influx_db)
                    .await?;
            }
        }

        Ok(())
    }
}
