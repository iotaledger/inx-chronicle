// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::payload as stardust;
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

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    pub(crate) use super::milestone::test::get_test_milestone_id;
    use super::{milestone, tagged_data, transaction, treasury_transaction, *};

    #[test]
    fn test_payload_bson() {
        let payload = get_test_transaction_payload();
        let bson = to_bson(&payload).unwrap();
        from_bson::<Payload>(bson).unwrap();

        let payload = get_test_milestone_payload();
        let bson = to_bson(&payload).unwrap();
        from_bson::<Payload>(bson).unwrap();

        let payload = get_test_treasury_transaction_payload();
        let bson = to_bson(&payload).unwrap();
        from_bson::<Payload>(bson).unwrap();

        let payload = get_test_tagged_data_payload();
        let bson = to_bson(&payload).unwrap();
        from_bson::<Payload>(bson).unwrap();
    }

    pub(crate) fn get_test_transaction_payload() -> Payload {
        Payload::Transaction(Box::new(transaction::test::get_test_transaction_payload()))
    }

    pub(crate) fn get_test_milestone_payload() -> Payload {
        Payload::Milestone(Box::new(milestone::test::get_test_milestone_payload()))
    }

    pub(crate) fn get_test_treasury_transaction_payload() -> Payload {
        Payload::TreasuryTransaction(Box::new(
            treasury_transaction::test::get_test_treasury_transaction_payload(),
        ))
    }

    pub(crate) fn get_test_tagged_data_payload() -> Payload {
        Payload::TaggedData(Box::new(tagged_data::test::get_test_tagged_data_payload()))
    }
}
