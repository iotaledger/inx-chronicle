// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust as bee;
use packable::PackableExt;

use super::InxError;
use crate::{
    maybe_missing,
    types::{
        ledger::{LedgerOutput, LedgerSpent},
        tangle::MilestoneIndex, context::{TryFromWithContext, TryIntoWithContext},
    },
};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnspentOutputMessage {
    pub ledger_index: MilestoneIndex,
    pub output: LedgerOutput,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    pub milestone_index: MilestoneIndex,
    pub consumed_count: usize,
    pub created_count: usize,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerUpdateMessage {
    Consumed(LedgerSpent),
    Created(LedgerOutput),
    Begin(Marker),
    End(Marker),
}

impl LedgerUpdateMessage {
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

impl From<inx::proto::ledger_update::Marker> for LedgerUpdateMessage {
    fn from(value: inx::proto::ledger_update::Marker) -> Self {
        use inx::proto::ledger_update::marker::MarkerType as proto;
        match value.marker_type() {
            proto::Begin => Self::Begin(value.into()),
            proto::End => Self::End(value.into()),
        }
    }
}

impl TryFrom<inx::proto::LedgerUpdate> for LedgerUpdateMessage {
    type Error = InxError;

    fn try_from(value: inx::proto::LedgerUpdate) -> Result<Self, Self::Error> {
        use inx::proto::ledger_update::Op as proto;
        Ok(match maybe_missing!(value.op) {
            proto::BatchMarker(marker) => marker.into(),
            proto::Consumed(consumed) => LedgerUpdateMessage::Consumed(consumed.try_into()?),
            proto::Created(created) => LedgerUpdateMessage::Created(created.try_into()?),
        })
    }
}

impl TryFrom<inx::proto::UnspentOutput> for UnspentOutputMessage {
    type Error = InxError;

    fn try_from(value: inx::proto::UnspentOutput) -> Result<Self, Self::Error> {
        Ok(Self {
            ledger_index: value.ledger_index.into(),
            output: maybe_missing!(value.output).try_into()?,
        })
    }
}

impl TryFromWithContext<UnspentOutputMessage> for inx::proto::UnspentOutput {
    type Error = bee::Error;

    fn try_from_with_context(ctx: &bee::protocol::ProtocolParameters, value: UnspentOutputMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            ledger_index: value.ledger_index.0,
            output: Some(value.output.try_into_with_context(ctx)?),
        })
    }
}

impl TryFromWithContext<LedgerOutput> for inx::proto::LedgerOutput {
    type Error = bee::Error;

    fn try_from_with_context(ctx: &bee::protocol::ProtocolParameters, value: LedgerOutput) -> Result<Self, Self::Error> {
        let bee_output = bee::output::Output::try_from_with_context(ctx, value.output)?;
        
        Ok(Self {
            block_id: Some(value.block_id.into()),
            milestone_index_booked: value.booked.milestone_index.0,
            milestone_timestamp_booked: value.booked.milestone_timestamp.0,
            output: Some(inx::proto::RawOutput{ data: bee_output.pack_to_vec()}),
            output_id: Some(value.output_id.into()),
        })
    }
}
