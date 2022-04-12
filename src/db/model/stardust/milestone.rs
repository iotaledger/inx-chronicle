// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::{payload::milestone::MilestoneId, MessageId};
use packable::PackableExt;
use serde::{Deserialize, Serialize};

use crate::{db::Model, inx::InxError};

#[derive(Debug, Serialize, Deserialize)]
/// A record that stores information about a milestone.
pub struct Milestone {
    milestone_index: u32,
    milestone_timestamp: u32,
    message_id: MessageId,
    milestone_id: MilestoneId,
}

impl Model for Milestone {
    const COLLECTION: &'static str = "stardust_milestones";
}

impl TryFrom<inx::proto::Milestone> for Milestone {
    type Error = InxError;

    fn try_from(milestone: inx::proto::Milestone) -> Result<Self, Self::Error> {
        let milestone_index = milestone.milestone_index;
        let milestone_timestamp = milestone.milestone_timestamp;
        let mut message_id_bytes = milestone.message_id.ok_or(InxError::MissingField("message_id"))?.id;
        let mut milestone_id_bytes = milestone.milestone_id.ok_or(InxError::MissingField("milestone_id"))?.id;

        let message_id =
            MessageId::unpack_verified(&mut message_id_bytes).map_err(|_| InxError::InvalidField("message_id"))?;

        let milestone_id = MilestoneId::unpack_verified(&mut milestone_id_bytes)
            .map_err(|_| InxError::InvalidField("milestone_id"))?;

        Ok(Milestone {
            milestone_index,
            milestone_timestamp,
            message_id,
            milestone_id,
        })
    }
}
