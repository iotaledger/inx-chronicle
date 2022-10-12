// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::{types::{tangle::MilestoneIndex, ledger::{LedgerSpent, LedgerOutput}}, maybe_missing};

use super::InxError;

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    pub milestone_index: MilestoneIndex,
    pub consumed_count: usize,
    pub created_count: usize,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerUpdate {
    Consumed(LedgerSpent),
    Created(LedgerOutput),
    Begin(Marker),
    End(Marker),
}

impl LedgerUpdate {
    /// If present, returns the contained `LedgerSpent` while consuming `self`.
    pub fn consumed(self) -> Option<LedgerSpent> {
        match self {
            Self::Consumed(ledger_spent) => Some(ledger_spent),
            _ => None,
        }
    }

    /// If present, returns the contained `LedgerOutput` while consuming `self`.
    pub fn created(self) -> Option<LedgerOutput> {
        match self {
            Self::Created(ledger_output) => Some(ledger_output),
            _ => None,
        }
    }

    /// If present, returns the `Marker` that denotes the beginning of a milestone while consuming `self`.
    pub fn begin(self) -> Option<Marker> {
        match self {
            Self::Begin(marker) => Some(marker),
            _ => None,
        }
    }

    /// If present, returns the `Marker` that denotes the end if present while consuming `self`.
    pub fn end(self) -> Option<Marker> {
        match self {
            Self::End(marker) => Some(marker),
            _ => None,
        }
    }
}

impl From<inx::proto::ledger_update::Marker> for Marker {
    fn from(value: inx::proto::ledger_update::Marker) -> Self {
        Self {
            milestone_index: value.milestone_index.into(),
            consumed_count: value.consumed_count as usize,
            created_count: value.created_count as usize,
        }
    }
}

impl From<inx::proto::ledger_update::Marker> for LedgerUpdate {
    fn from(value: inx::proto::ledger_update::Marker) -> Self {
        use inx::proto::ledger_update::marker::MarkerType as proto;
        match value.marker_type() {
            proto::Begin => Self::Begin(value.into()),
            proto::End => Self::End(value.into()),
        }
    }
}

impl TryFrom<inx::proto::LedgerUpdate> for LedgerUpdate {
    type Error = InxError;

    fn try_from(value: inx::proto::LedgerUpdate) -> Result<Self, Self::Error> {
        use inx::proto::ledger_update::Op as proto;
        Ok(match maybe_missing!(value.op) {
            proto::BatchMarker(marker) => marker.into(),
            proto::Consumed(consumed) => LedgerUpdate::Consumed(consumed.try_into()?),
            proto::Created(created) => LedgerUpdate::Created(created.try_into()?),
        })
    }
}
