// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use chronicle::{
    analytics::{Analytic, AnalyticsContext},
    db::{
        influxdb::{AnalyticsChoice, InfluxDb},
        mongodb::collections::{ApplicationStateCollection, OutputCollection},
        MongoDb,
    },
    inx::Inx,
    tangle::Slot,
};
use futures::TryStreamExt;
use iota_sdk::types::block::slot::SlotIndex;

use super::InxWorkerError;
use crate::{cli::analytics::AnalyticsState, inx::InxWorker};

pub struct AnalyticsInfo {
    analytics_choices: HashSet<AnalyticsChoice>,
    state: Option<AnalyticsState>,
    pub synced_index: SlotIndex,
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
                    .ok_or(InxWorkerError::MissingAppState)?,
            })
        } else {
            None
        })
    }
}

impl InxWorker {
    pub async fn update_analytics<'a>(
        &self,
        slot: &Slot<'a, Inx>,
        AnalyticsInfo {
            analytics_choices,
            state,
            ..
        }: &mut AnalyticsInfo,
    ) -> eyre::Result<()> {
        if let (Some(influx_db), analytics_choices) = (&self.influx_db, analytics_choices) {
            if influx_db.config().analytics_enabled {
                // Check if the protocol params changed (or we just started)
                if !matches!(&state, Some(state) if state.prev_protocol_params == slot.protocol_parameters) {
                    let ledger_state = self
                        .db
                        .collection::<OutputCollection>()
                        .get_unspent_output_stream(slot.slot_index() - 1)
                        .await?
                        .try_collect::<Vec<_>>()
                        .await?;

                    let analytics = analytics_choices
                        .iter()
                        .map(|choice| Analytic::init(choice, &slot.protocol_parameters, &ledger_state))
                        .collect::<Vec<_>>();
                    *state = Some(AnalyticsState {
                        analytics,
                        prev_protocol_params: slot.protocol_parameters.clone(),
                    });
                }

                // Unwrap: safe because we guarantee it is initialized above
                slot.update_analytics(&mut state.as_mut().unwrap().analytics, influx_db)
                    .await?;
            }
        }

        Ok(())
    }
}
