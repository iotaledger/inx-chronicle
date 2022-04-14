// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust as stardust;
use serde::{Deserialize, Serialize};

use super::Model;

/// Model for the [`Message`].
#[derive(Serialize, Deserialize)]
pub struct Message {
    /// The [`MessageId`](stardust::MessageId) of the message.
    pub message_id: stardust::MessageId,
    /// The actual rich representation of the [`Message`](stardust::Message).
    pub message: stardust::Message,
    /// The raw bytes of the message.
    pub raw: Vec<u8>,
}

impl Model for Message {
    const COLLECTION: &'static str = "stardust_messages";
}

/// Model
#[derive(Serialize, Deserialize)]
pub struct Milestone {
    /// The milestone index.
    pub milestone_index: u32,
    /// The timestamp of the milestone.
    pub milestone_timestamp: u32,
    /// The [`MessageId`](stardust::MessageId) of the milestone.
    pub message_id: stardust::MessageId,
    /// The [`MilestoneId`](stardust::payload::milestone::MilestoneId) of the milestone.
    pub milestone_id: stardust::payload::milestone::MilestoneId,
}

impl Model for Milestone {
    const COLLECTION: &'static str = "stardust_milestones";
}
