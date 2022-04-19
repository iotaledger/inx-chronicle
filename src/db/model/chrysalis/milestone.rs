// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_chrysalis::MessageId;
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

use crate::db::model::Model;

/// A milestone's metadata.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MilestoneRecord {
    /// The milestone index.
    pub milestone_index: u32,
    /// The timestamp of the milestone.
    pub milestone_timestamp: DateTime,
    /// The [`MessageId`] of the milestone.
    pub message_id: MessageId,
}

impl Model for MilestoneRecord {
    const COLLECTION: &'static str = "chrysalis_milestones";

    fn key(&self) -> mongodb::bson::Document {
        doc! { "milestone_index": self.milestone_index }
    }
}
