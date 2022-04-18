// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_message_chrysalis::MessageId;
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

use crate::db::model::{ConversionError, Model};

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

impl TryFrom<serde_json::Value> for MilestoneRecord {
    type Error = ConversionError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        log::warn!("milestone json: {}", value);
        let milestone = value
            .get("milestone")
            .ok_or(ConversionError::MissingField("milestone"))?;
        Ok(Self {
            milestone_index: value
                .get("index")
                .ok_or(ConversionError::MissingField("index"))?
                .as_u64()
                .ok_or(ConversionError::InvalidField("index"))? as u32,
            milestone_timestamp: DateTime::from_millis(
                milestone
                    .get("timestamp")
                    .ok_or(ConversionError::MissingField("timestamp"))?
                    .as_u64()
                    .ok_or(ConversionError::InvalidField("timestamp"))? as i64
                    * 1000,
            ),
            message_id: MessageId::from_str(
                milestone
                    .get("message_id")
                    .ok_or(ConversionError::MissingField("message_id"))?
                    .as_str()
                    .ok_or(ConversionError::InvalidField("message_id"))?,
            )
            .unwrap(),
        })
    }
}
