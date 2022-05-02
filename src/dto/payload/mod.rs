// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Payload {
    Transaction(Box<TransactionPayload>),
    Milestone(Box<MilestonePayload>),
    TreasuryTransaction(Box<TreasuryTransactionPayload>),
    TaggedData(Box<TaggedDataPayload>),
}
