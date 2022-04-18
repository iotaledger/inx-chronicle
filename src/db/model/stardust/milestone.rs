// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::{payload::milestone::MilestoneId, MessageId};
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

use crate::db::model::{stardust::message::message_id_from_inx, ConversionError, Model};

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
    const COLLECTION: &'static str = "stardust_milestones";

    fn key(&self) -> mongodb::bson::Document {
        doc! { "milestone_index": self.milestone_index }
    }
}

impl TryFrom<inx::proto::Milestone> for MilestoneRecord {
    type Error = ConversionError;

    fn try_from(value: inx::proto::Milestone) -> Result<Self, Self::Error> {
        Ok(Self {
            milestone_index: value.milestone_index,
            milestone_timestamp: DateTime::from_millis(value.milestone_timestamp as i64 * 1000),
            message_id: message_id_from_inx(value.message_id.ok_or(ConversionError::MissingField("message_id"))?)?,
            milestone_id: milestone_id_from_inx(
                value
                    .milestone_id
                    .ok_or(ConversionError::MissingField("milestone_id"))?,
            )?,
        })
    }
}

pub(crate) fn milestone_id_from_inx(value: inx::proto::MilestoneId) -> Result<MilestoneId, ConversionError> {
    Ok(MilestoneId::from(
        <[u8; MilestoneId::LENGTH]>::try_from(value.id).map_err(|_| ConversionError::InvalidBufferLength)?,
    ))
}
