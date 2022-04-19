// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::payload::milestone::MilestoneId;
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

use crate::db::model::Model;

/// A milestone's metadata.
#[derive(Serialize, Deserialize)]
pub struct MilestoneRecord {
    /// The milestone index.
    pub milestone_index: u32,
    /// The timestamp of the milestone.
    pub milestone_timestamp: DateTime,
    /// The [`MilestoneId`] of the milestone.
    pub milestone_id: MilestoneId,
}

impl Model for MilestoneRecord {
    const COLLECTION: &'static str = "stardust_milestones";

    fn key(&self) -> mongodb::bson::Document {
        doc! { "milestone_index": self.milestone_index }
    }
}

impl TryFrom<inx::proto::Milestone> for MilestoneRecord {
    type Error = inx::Error;

    fn try_from(value: inx::proto::Milestone) -> Result<Self, Self::Error> {
        let milestone = inx::Milestone::try_from(value)?;
        Ok(Self {
            milestone_index: milestone.milestone_info.milestone_index,
            milestone_timestamp: DateTime::from_millis(milestone.milestone_info.milestone_timestamp as i64 * 1000),
            milestone_id: milestone.milestone_info.milestone_id,
        })
    }
}
