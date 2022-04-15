// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_chrysalis::{payload::milestone::MilestoneId, prelude::MILESTONE_ID_LENGTH, MessageId};
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

use crate::db::model::{chrysalis::message::message_id_from_inx, InxConversionError, Model};

/// A milestone's metadata.
#[derive(Serialize, Deserialize)]
pub struct MilestoneRecord {
    /// The milestone index.
    pub milestone_index: u32,
    /// The timestamp of the milestone.
    pub milestone_timestamp: DateTime,
    /// The [`MessageId`] of the milestone.
    pub message_id: MessageId,
    /// The [`MilestoneId`] of the milestone.
    pub milestone_id: MilestoneId,
}

impl Model for MilestoneRecord {
    const COLLECTION: &'static str = "chrysalis_milestones";

    fn key(&self) -> mongodb::bson::Document {
        doc! { "milestone_index": self.milestone_index }
    }
}

impl TryFrom<inx::proto::Milestone> for MilestoneRecord {
    type Error = InxConversionError;

    fn try_from(value: inx::proto::Milestone) -> Result<Self, Self::Error> {
        Ok(Self {
            milestone_index: value.milestone_index,
            milestone_timestamp: DateTime::from_millis(value.milestone_timestamp as i64 * 1000),
            message_id: message_id_from_inx(value.message_id.ok_or(InxConversionError::MissingField("message_id"))?)?,
            milestone_id: milestone_id_from_inx(
                value
                    .milestone_id
                    .ok_or(InxConversionError::MissingField("milestone_id"))?,
            )?,
        })
    }
}

pub(crate) fn milestone_id_from_inx(value: inx::proto::MilestoneId) -> Result<MilestoneId, InxConversionError> {
    Ok(MilestoneId::from(
        <[u8; MILESTONE_ID_LENGTH]>::try_from(value.id).map_err(|_| InxConversionError::InvalidBufferLength)?,
    ))
}
