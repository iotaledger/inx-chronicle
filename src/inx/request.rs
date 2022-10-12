// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! This module offers convenience functionality to request per-milestone information via INX.

use std::ops::{Bound, RangeBounds};

use crate::types::{tangle::MilestoneIndex, stardust::block::payload::MilestoneId};
use inx::proto;

/// A request for a milestone that can either be a [`MilestoneIndex`] or a [`MilestoneId`].
pub enum MilestoneRequest {
    /// Request milestone information by milestone index.
    MilestoneIndex(MilestoneIndex),
    /// Request milestone information by milestone id.
    MilestoneId(MilestoneId),
}

impl From<MilestoneRequest> for proto::MilestoneRequest {
    fn from(value: MilestoneRequest) -> Self {
        match value {
            MilestoneRequest::MilestoneIndex(MilestoneIndex(milestone_index)) => Self {
                milestone_index,
                milestone_id: None,
            },
            MilestoneRequest::MilestoneId(milestone_id) => Self {
                milestone_index: 0,
                milestone_id: Some(inx::proto::MilestoneId{id: milestone_id.0.to_vec()}),
            },
        }
    }
}

impl<T: Into<u32>> From<T> for MilestoneRequest {
    fn from(value: T) -> Self {
        Self::MilestoneIndex(MilestoneIndex(value.into()))
    }
}

fn to_milestone_range_request<T, I>(range: T) -> proto::MilestoneRangeRequest
where
    T: RangeBounds<I>,
    I: Into<u32> + Copy,
{
    let start_milestone_index = match range.start_bound() {
        Bound::Included(&idx) => idx.into(),
        Bound::Excluded(&idx) => idx.into() + 1,
        Bound::Unbounded => 0,
    };
    let end_milestone_index = match range.end_bound() {
        Bound::Included(&idx) => idx.into(),
        Bound::Excluded(&idx) => idx.into() - 1,
        Bound::Unbounded => 0,
    };
    proto::MilestoneRangeRequest {
        start_milestone_index,
        end_milestone_index,
    }
}

/// A request for a range of milestones by [`MilestoneIndex`].
#[derive(Clone, Debug, PartialEq)]
pub struct MilestoneRangeRequest(proto::MilestoneRangeRequest);

impl<T> From<T> for MilestoneRangeRequest
where
    T: RangeBounds<u32>,
{
    fn from(value: T) -> MilestoneRangeRequest {
        MilestoneRangeRequest(to_milestone_range_request(value))
    }
}

impl From<MilestoneRangeRequest> for proto::MilestoneRangeRequest {
    fn from(value: MilestoneRangeRequest) -> Self {
        value.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn exclusive() {
        let range = MilestoneRangeRequest::from(17..43);
        assert_eq!(
            range,
            MilestoneRangeRequest(proto::MilestoneRangeRequest {
                start_milestone_index: 17,
                end_milestone_index: 42
            })
        );
    }

    #[test]
    fn inclusive() {
        let range = MilestoneRangeRequest::from(17..=42);
        assert_eq!(
            range,
            MilestoneRangeRequest(proto::MilestoneRangeRequest {
                start_milestone_index: 17,
                end_milestone_index: 42
            })
        );
    }
}
