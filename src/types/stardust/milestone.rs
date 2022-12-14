// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::{Add, Deref, DerefMut, Sub};
use mongodb::bson::{doc, Bson};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// The Unix timestamp of a milestone.
#[derive(
    Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Add, Sub, Deref, DerefMut,
)]
#[serde(transparent)]
pub struct MilestoneTimestamp(pub u32);

impl From<u32> for MilestoneTimestamp {
    fn from(value: u32) -> Self {
        MilestoneTimestamp(value)
    }
}

impl From<MilestoneTimestamp> for Bson {
    fn from(value: MilestoneTimestamp) -> Self {
        Bson::from(value.0)
    }
}

impl TryFrom<MilestoneTimestamp> for OffsetDateTime {
    type Error = time::Error;

    fn try_from(value: MilestoneTimestamp) -> Result<Self, Self::Error> {
        OffsetDateTime::from_unix_timestamp(value.0 as i64).map_err(time::Error::from)
    }
}

impl From<OffsetDateTime> for MilestoneTimestamp {
    fn from(value: OffsetDateTime) -> Self {
        MilestoneTimestamp(value.unix_timestamp() as u32)
    }
}

#[cfg(any(feature = "analytics", feature = "metrics"))]
impl From<MilestoneTimestamp> for influxdb::Timestamp {
    fn from(value: MilestoneTimestamp) -> Self {
        Self::Seconds(value.0 as _)
    }
}

#[cfg(test)]
mod test {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn to_from_offset_date_time() {
        let date = datetime!(2022-12-08 0:00).assume_utc();
        let milestone_timestamp = MilestoneTimestamp::from(date);
        assert_eq!(
            milestone_timestamp,
            MilestoneTimestamp(1670457600),
            "convert to `MilestoneTimestamp`"
        );
        assert_eq!(
            OffsetDateTime::try_from(milestone_timestamp).unwrap(),
            date,
            "convert from `MilestoneTimestamp`"
        );
    }
}
