// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use influxdb::InfluxDbWriteable;
use mongodb::error::Error;

use super::{Analytic, Measurement, PerMilestone};
use crate::{
    db::{collections::ProtocolUpdateCollection, MongoDb},
    types::{
        stardust::milestone::MilestoneTimestamp,
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

/// Computes the statistics about the token claiming process.
#[derive(Debug)]
pub struct ProtocolParametersAnalytics;

#[async_trait]
impl Analytic for ProtocolParametersAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Box<dyn Measurement>, Error>> {
        let res = db
            .collection::<ProtocolUpdateCollection>()
            .get_protocol_parameters_for_milestone_index(milestone_index)
            .await;

        match res {
            Ok(Some(p)) => Some(Ok(Box::new(PerMilestone {
                milestone_index,
                milestone_timestamp,
                measurement: p.parameters,
            }))),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}

impl Measurement for PerMilestone<ProtocolParameters> {
    fn into_write_query(&self) -> influxdb::WriteQuery {
        influxdb::Timestamp::from(self.milestone_timestamp)
            .into_query("stardust_protocol_params")
            .add_field("milestone_index", self.milestone_index)
            .add_field("token_supply", self.measurement.token_supply)
            .add_field("min_pow_score", self.measurement.min_pow_score)
            .add_field("below_max_depth", self.measurement.below_max_depth)
            .add_field("v_byte_cost", self.measurement.rent_structure.v_byte_cost)
            .add_field("v_byte_factor_key", self.measurement.rent_structure.v_byte_factor_key)
            .add_field("v_byte_factor_data", self.measurement.rent_structure.v_byte_factor_data)
    }
}
