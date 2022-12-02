// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use mongodb::error::Error;

use super::{Analytic, Measurement, PerMilestone};
use crate::{
    db::{collections::ProtocolUpdateCollection, MongoDb},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
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
    ) -> Option<Result<Measurement, Error>> {
        let res = db
            .collection::<ProtocolUpdateCollection>()
            .get_protocol_parameters_for_milestone_index(milestone_index)
            .await;

        match res {
            Ok(Some(p)) => Some(Ok(Measurement::ProtocolParameters(PerMilestone {
                milestone_index,
                milestone_timestamp,
                inner: p.parameters,
            }))),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}
