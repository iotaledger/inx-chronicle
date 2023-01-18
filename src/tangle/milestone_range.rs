// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::tangle::MilestoneIndex;

#[derive(Copy, Clone, Debug)]
pub struct MilestoneRange {
    pub start: MilestoneIndex,
    pub end: MilestoneIndex,
}

impl<T> From<T> for MilestoneRange
where
    T: std::ops::RangeBounds<MilestoneIndex>,
{
    fn from(value: T) -> MilestoneRange {
        use std::ops::Bound;
        let start = match value.start_bound() {
            Bound::Included(&idx) => idx,
            Bound::Excluded(&idx) => idx + 1,
            Bound::Unbounded => 0.into(),
        };
        let end = match value.end_bound() {
            Bound::Included(&idx) => idx,
            Bound::Excluded(&idx) => idx - 1,
            Bound::Unbounded => u32::MAX.into(),
        };
        MilestoneRange { start, end }
    }
}

impl Iterator for MilestoneRange {
    type Item = MilestoneIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let res = self.start;
            self.start += 1;
            Some(res)
        } else {
            None
        }
    }
}
