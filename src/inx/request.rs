// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! This module offers convenience functionality to request per-milestone information via INX.

use std::ops::{Bound, RangeBounds};

use inx::proto;

// /// A request for a milestone that can either be a [`MilestoneIndex`] or a [`MilestoneId`].
// pub enum MilestoneRequest {
//     /// Request milestone information by milestone index.
//     MilestoneIndex(MilestoneIndex),
//     /// Request milestone information by milestone id.
//     MilestoneId(MilestoneId),
// }

// impl From<MilestoneRequest> for proto::MilestoneRequest {
//     fn from(value: MilestoneRequest) -> Self {
//         match value {
//             MilestoneRequest::MilestoneIndex(MilestoneIndex(milestone_index)) => Self {
//                 milestone_index,
//                 milestone_id: None,
//             },
//             MilestoneRequest::MilestoneId(milestone_id) => Self {
//                 milestone_index: 0,
//                 milestone_id: Some(inx::proto::MilestoneId {
//                     id: milestone_id.0.to_vec(),
//                 }),
//             },
//         }
//     }
// }

// impl<T: Into<u32>> From<T> for MilestoneRequest {
//     fn from(value: T) -> Self {
//         Self::MilestoneIndex(MilestoneIndex(value.into()))
//     }
// }

fn to_slot_range_request<T, I>(range: T) -> proto::SlotRangeRequest
where
    T: RangeBounds<I>,
    I: Into<u32> + Copy,
{
    let start_slot = match range.start_bound() {
        Bound::Included(&idx) => idx.into(),
        Bound::Excluded(&idx) => idx.into() + 1,
        Bound::Unbounded => 0,
    };
    let end_slot = match range.end_bound() {
        Bound::Included(&idx) => idx.into(),
        Bound::Excluded(&idx) => idx.into() - 1,
        Bound::Unbounded => 0,
    };
    proto::SlotRangeRequest { start_slot, end_slot }
}

/// A request for a range of slots by [`SlotIndex`](iota_sdk::types::block::slot::SlotIndex).
#[derive(Clone, Debug, PartialEq)]
pub struct SlotRangeRequest(proto::SlotRangeRequest);

impl<T> From<T> for SlotRangeRequest
where
    T: RangeBounds<u32>,
{
    fn from(value: T) -> SlotRangeRequest {
        SlotRangeRequest(to_slot_range_request(value))
    }
}

impl SlotRangeRequest {
    /// Convert any range that can be interpreted as a range request.
    pub fn from_range<T, I>(range: T) -> Self
    where
        T: RangeBounds<I>,
        I: Into<u32> + Copy,
    {
        Self(to_slot_range_request(range))
    }
}

impl From<SlotRangeRequest> for proto::SlotRangeRequest {
    fn from(value: SlotRangeRequest) -> Self {
        value.0
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn exclusive() {
        let range = SlotRangeRequest::from(17..43);
        assert_eq!(
            range,
            SlotRangeRequest(proto::SlotRangeRequest {
                start_slot: 17,
                end_slot: 42
            })
        );
    }

    #[test]
    fn inclusive() {
        let range = SlotRangeRequest::from(17..=42);
        assert_eq!(
            range,
            SlotRangeRequest(proto::SlotRangeRequest {
                start_slot: 17,
                end_slot: 42
            })
        );
    }
}
