// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use inx::proto;
use iota_types::block as iota;

use super::{raw::RawMessage, InxError, RawProtocolParametersMessage};
use crate::{
    maybe_missing,
    types::stardust::tangle::{block::payload::MilestoneId, milestone::MilestoneIndex},
};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneMessage {
    /// Information about the milestone.
    pub milestone_info: MilestoneInfoMessage,
    /// The raw bytes of the milestone. Note that this is not a [`iota::payload::milestone::MilestonePayload`], but
    /// rather a [`iota::payload::Payload`] and still needs to be unpacked.
    pub milestone: RawMessage<iota::payload::Payload>,
}

impl TryFrom<proto::Milestone> for MilestoneMessage {
    type Error = InxError;

    fn try_from(value: proto::Milestone) -> Result<Self, Self::Error> {
        Ok(Self {
            milestone_info: maybe_missing!(value.milestone_info).try_into()?,
            milestone: maybe_missing!(value.milestone).data.into(),
        })
    }
}

impl TryFrom<MilestoneMessage> for proto::Milestone {
    type Error = InxError;

    fn try_from(value: MilestoneMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            milestone_info: Some(value.milestone_info.try_into()?),
            milestone: Some(value.milestone.into()),
        })
    }
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneAndProtocolParametersMessage {
    pub milestone: MilestoneMessage,
    pub current_protocol_parameters: RawProtocolParametersMessage,
}

impl TryFrom<proto::MilestoneAndProtocolParameters> for MilestoneAndProtocolParametersMessage {
    type Error = InxError;

    fn try_from(value: proto::MilestoneAndProtocolParameters) -> Result<Self, Self::Error> {
        Ok(Self {
            milestone: maybe_missing!(value.milestone).try_into()?,
            current_protocol_parameters: maybe_missing!(value.current_protocol_parameters).into(),
        })
    }
}

impl TryFrom<MilestoneAndProtocolParametersMessage> for proto::MilestoneAndProtocolParameters {
    type Error = InxError;

    fn try_from(value: MilestoneAndProtocolParametersMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            milestone: Some(value.milestone.try_into()?),
            current_protocol_parameters: Some(value.current_protocol_parameters.into()),
        })
    }
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneInfoMessage {
    /// The [`MilestoneId`] of the milestone.
    pub milestone_id: Option<MilestoneId>,
    /// The milestone index.
    pub milestone_index: MilestoneIndex,
    /// The timestamp of the milestone.
    pub milestone_timestamp: u32,
}

impl TryFrom<proto::MilestoneInfo> for MilestoneInfoMessage {
    type Error = InxError;

    fn try_from(value: proto::MilestoneInfo) -> Result<Self, Self::Error> {
        Ok(MilestoneInfoMessage {
            milestone_id: value.milestone_id.map(TryInto::try_into).transpose()?,
            milestone_index: value.milestone_index.into(),
            milestone_timestamp: value.milestone_timestamp,
        })
    }
}

impl TryFrom<MilestoneInfoMessage> for proto::MilestoneInfo {
    type Error = InxError;

    fn try_from(value: MilestoneInfoMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            milestone_id: value.milestone_id.map(Into::into),
            milestone_index: value.milestone_index.0,
            milestone_timestamp: value.milestone_timestamp,
        })
    }
}
