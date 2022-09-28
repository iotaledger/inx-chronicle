// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Payload`] types.

use std::borrow::Borrow;

use bee_block_stardust::payload as bee;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod milestone;
pub mod tagged_data;
pub mod transaction;
pub mod treasury_transaction;

pub use self::{
    milestone::{MilestoneId, MilestonePayload},
    tagged_data::TaggedDataPayload,
    transaction::{TransactionEssence, TransactionId, TransactionPayload},
    treasury_transaction::TreasuryTransactionPayload,
};
use crate::types::context::{TryFromWithContext, TryIntoWithContext};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Payload {
    Transaction(Box<TransactionPayload>),
    Milestone(Box<MilestonePayload>),
    TreasuryTransaction(Box<TreasuryTransactionPayload>),
    TaggedData(Box<TaggedDataPayload>),
}

impl Payload {
    pub fn kind(&self) -> &'static str {
        match self {
            Payload::Transaction(_) => "transaction",
            Payload::Milestone(_) => "milestone",
            Payload::TreasuryTransaction(_) => "treasury_transaction",
            Payload::TaggedData(_) => "tagged_data",
        }
    }
}

impl<T: Borrow<bee::Payload>> From<T> for Payload {
    fn from(value: T) -> Self {
        match value.borrow() {
            bee::Payload::Transaction(p) => Self::Transaction(Box::new(p.as_ref().into())),
            bee::Payload::Milestone(p) => Self::Milestone(Box::new(p.as_ref().into())),
            bee::Payload::TreasuryTransaction(p) => Self::TreasuryTransaction(Box::new(p.as_ref().into())),
            bee::Payload::TaggedData(p) => Self::TaggedData(Box::new(p.as_ref().into())),
        }
    }
}

impl TryFromWithContext<Payload> for bee::Payload {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(
        ctx: &bee_block_stardust::protocol::ProtocolParameters,
        value: Payload,
    ) -> Result<Self, Self::Error> {
        Ok(match value {
            Payload::Transaction(p) => bee::Payload::Transaction(Box::new((*p).try_into_with_context(ctx)?)),
            Payload::Milestone(p) => bee::Payload::Milestone(Box::new((*p).try_into_with_context(ctx)?)),
            Payload::TreasuryTransaction(p) => {
                bee::Payload::TreasuryTransaction(Box::new((*p).try_into_with_context(ctx)?))
            }
            Payload::TaggedData(p) => bee::Payload::TaggedData(Box::new((*p).try_into()?)),
        })
    }
}

#[derive(Debug, Error)]
#[error("wrong payload requested. expected {expected}, found: {found}")]
pub struct WrongPayloadError {
    expected: &'static str,
    found: &'static str,
}

macro_rules! impl_coerce_payload {
    ($kind:literal, $t:ty, $var:ident) => {
        impl TryFrom<Payload> for $t {
            type Error = WrongPayloadError;

            fn try_from(value: Payload) -> Result<Self, Self::Error> {
                if let Payload::$var(payload) = value {
                    Ok(*payload)
                } else {
                    Err(WrongPayloadError {
                        expected: $kind,
                        found: value.kind(),
                    })
                }
            }
        }
    };
}
impl_coerce_payload!("transaction", TransactionPayload, Transaction);
impl_coerce_payload!("milestone", MilestonePayload, Milestone);
impl_coerce_payload!("treasury_transaction", TreasuryTransactionPayload, TreasuryTransaction);
impl_coerce_payload!("tagged_data", TaggedDataPayload, TaggedData);

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::number::rand_number_range;

    use super::*;

    impl Payload {
        /// Generates a random [`Payload`].
        pub fn rand(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            match rand_number_range(0..4) {
                0 => Self::rand_transaction(ctx),
                1 => Self::rand_milestone(ctx),
                2 => Self::rand_tagged_data(),
                3 => Self::rand_treasury_transaction(ctx),
                _ => unreachable!(),
            }
        }

        /// Generates a random, optional [`Payload`].
        pub fn rand_opt(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Option<Self> {
            match rand_number_range(0..5) {
                0 => Self::rand_transaction(ctx).into(),
                1 => Self::rand_milestone(ctx).into(),
                2 => Self::rand_tagged_data().into(),
                3 => Self::rand_treasury_transaction(ctx).into(),
                4 => None,
                _ => unreachable!(),
            }
        }

        /// Generates a random transaction [`Payload`].
        pub fn rand_transaction(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            Self::Transaction(Box::new(TransactionPayload::rand(ctx)))
        }

        /// Generates a random milestone [`Payload`].
        pub fn rand_milestone(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            Self::Milestone(Box::new(MilestonePayload::rand(ctx)))
        }

        /// Generates a random tagged data [`Payload`].
        pub fn rand_tagged_data() -> Self {
            Self::TaggedData(Box::new(TaggedDataPayload::rand()))
        }

        /// Generates a random treasury transaction [`Payload`].
        pub fn rand_treasury_transaction(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            Self::TreasuryTransaction(Box::new(TreasuryTransactionPayload::rand(ctx)))
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{doc, from_bson, to_bson, to_document};

    use super::*;

    #[test]
    fn test_transaction_payload_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let payload = Payload::rand_transaction(&ctx);
        let mut bson = to_bson(&payload).unwrap();
        // Need to re-add outputs as they are not serialized
        let outputs_doc = if let Payload::Transaction(payload) = &payload {
            let TransactionEssence::Regular { outputs, .. } = &payload.essence;
            doc! { "outputs": outputs.iter().map(to_document).collect::<Result<Vec<_>, _>>().unwrap() }
        } else {
            unreachable!();
        };
        let doc = bson.as_document_mut().unwrap().get_document_mut("essence").unwrap();
        doc.extend(outputs_doc);
        assert_eq!(payload, from_bson::<Payload>(bson).unwrap());
    }

    #[test]
    fn test_milestone_payload_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let payload = Payload::rand_milestone(&ctx);
        bee::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<Payload>(bson).unwrap());
    }

    #[test]
    fn test_treasury_transaction_payload_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let payload = Payload::rand_treasury_transaction(&ctx);
        bee::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<Payload>(bson).unwrap());
    }

    #[test]
    fn test_tagged_data_payload_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let payload = Payload::rand_tagged_data();
        bee::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<Payload>(bson).unwrap());
    }
}
