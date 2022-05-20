// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::payload as stardust;
use serde::{Deserialize, Serialize};

mod milestone;
mod tagged_data;
mod transaction;
mod treasury_transaction;

pub use self::{
    milestone::{MilestoneEssence, MilestoneId, MilestoneIndex, MilestoneOption, MilestonePayload},
    tagged_data::TaggedDataPayload,
    transaction::{TransactionEssence, TransactionId, TransactionPayload},
    treasury_transaction::TreasuryTransactionPayload,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Payload {
    #[serde(rename = "transaction")]
    Transaction(Box<TransactionPayload>),
    #[serde(rename = "milestone")]
    Milestone(Box<MilestonePayload>),
    #[serde(rename = "treasury_transaction")]
    TreasuryTransaction(Box<TreasuryTransactionPayload>),
    #[serde(rename = "tagged_data")]
    TaggedData(Box<TaggedDataPayload>),
}

impl From<&stardust::Payload> for Payload {
    fn from(value: &stardust::Payload) -> Self {
        match value {
            stardust::Payload::Transaction(p) => Self::Transaction(Box::new(p.as_ref().into())),
            stardust::Payload::Milestone(p) => Self::Milestone(Box::new(p.as_ref().into())),
            stardust::Payload::TreasuryTransaction(p) => Self::TreasuryTransaction(Box::new(p.as_ref().into())),
            stardust::Payload::TaggedData(p) => Self::TaggedData(Box::new(p.as_ref().into())),
        }
    }
}

impl TryFrom<Payload> for stardust::Payload {
    type Error = crate::types::error::Error;

    fn try_from(value: Payload) -> Result<Self, Self::Error> {
        Ok(match value {
            Payload::Transaction(p) => stardust::Payload::Transaction(Box::new((*p).try_into()?)),
            Payload::Milestone(p) => stardust::Payload::Milestone(Box::new((*p).try_into()?)),
            Payload::TreasuryTransaction(p) => stardust::Payload::TreasuryTransaction(Box::new((*p).try_into()?)),
            Payload::TaggedData(p) => stardust::Payload::TaggedData(Box::new((*p).try_into()?)),
        })
    }
}
