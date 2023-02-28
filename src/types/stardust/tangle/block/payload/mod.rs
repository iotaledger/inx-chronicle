// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Payload`] types.

use std::borrow::Borrow;

use iota_types::block::payload as iota;
use serde::{Deserialize, Serialize};

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
use crate::types::stardust::protocol::{TryFromWithContext, TryIntoWithContext};

/// The different payloads of a [`Block`](crate::types::stardust::block::Block).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Payload {
    /// Signals a transaction of tokens.
    Transaction(Box<TransactionPayload>),
    /// Signals a milestone that acts as a checkpoint on which all nodes agree.
    Milestone(Box<MilestonePayload>),
    /// Signals a transaction that modifies the treasury.
    TreasuryTransaction(Box<TreasuryTransactionPayload>),
    /// Signals arbitrary data as a key-value pair.
    TaggedData(Box<TaggedDataPayload>),
}

impl<T: Borrow<iota::Payload>> From<T> for Payload {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::Payload::Transaction(p) => Self::Transaction(Box::new(p.as_ref().into())),
            iota::Payload::Milestone(p) => Self::Milestone(Box::new(p.as_ref().into())),
            iota::Payload::TreasuryTransaction(p) => Self::TreasuryTransaction(Box::new(p.as_ref().into())),
            iota::Payload::TaggedData(p) => Self::TaggedData(Box::new(p.as_ref().into())),
        }
    }
}

impl TryFromWithContext<Payload> for iota::Payload {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: Payload,
    ) -> Result<Self, Self::Error> {
        Ok(match value {
            Payload::Transaction(p) => iota::Payload::Transaction(Box::new((*p).try_into_with_context(ctx)?)),
            Payload::Milestone(p) => iota::Payload::Milestone(Box::new((*p).try_into_with_context(ctx)?)),
            Payload::TreasuryTransaction(p) => {
                iota::Payload::TreasuryTransaction(Box::new((*p).try_into_with_context(ctx)?))
            }
            Payload::TaggedData(p) => iota::Payload::TaggedData(Box::new((*p).try_into()?)),
        })
    }
}

impl From<Payload> for iota::dto::PayloadDto {
    fn from(value: Payload) -> Self {
        match value {
            Payload::Transaction(p) => Self::Transaction(Box::new((*p).into())),
            Payload::Milestone(p) => Self::Milestone(Box::new((*p).into())),
            Payload::TreasuryTransaction(p) => Self::TreasuryTransaction(Box::new((*p).into())),
            Payload::TaggedData(p) => Self::TaggedData(Box::new((*p).into())),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::number::rand_number_range;

    use super::*;

    impl Payload {
        /// Generates a random [`Payload`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            match rand_number_range(0..4) {
                0 => Self::rand_transaction(ctx),
                1 => Self::rand_milestone(ctx),
                2 => Self::rand_tagged_data(),
                3 => Self::rand_treasury_transaction(ctx),
                _ => unreachable!(),
            }
        }

        /// Generates a random, optional [`Payload`].
        pub fn rand_opt(ctx: &iota_types::block::protocol::ProtocolParameters) -> Option<Self> {
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
        pub fn rand_transaction(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self::Transaction(Box::new(TransactionPayload::rand(ctx)))
        }

        /// Generates a random milestone [`Payload`].
        pub fn rand_milestone(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self::Milestone(Box::new(MilestonePayload::rand(ctx)))
        }

        /// Generates a random tagged data [`Payload`].
        pub fn rand_tagged_data() -> Self {
            Self::TaggedData(Box::new(TaggedDataPayload::rand()))
        }

        /// Generates a random treasury transaction [`Payload`].
        pub fn rand_treasury_transaction(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
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
        let ctx = iota_types::block::protocol::protocol_parameters();
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
        assert_eq!(
            bson.as_document().unwrap().get_str("kind").unwrap(),
            TransactionPayload::KIND
        );
        assert_eq!(payload, from_bson::<Payload>(bson).unwrap());
    }

    #[test]
    fn test_milestone_payload_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
        let payload = Payload::rand_milestone(&ctx);
        iota::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(
            bson.as_document().unwrap().get_str("kind").unwrap(),
            MilestonePayload::KIND
        );
        assert_eq!(payload, from_bson::<Payload>(bson).unwrap());
    }

    #[test]
    fn test_treasury_transaction_payload_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
        let payload = Payload::rand_treasury_transaction(&ctx);
        iota::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(
            bson.as_document().unwrap().get_str("kind").unwrap(),
            TreasuryTransactionPayload::KIND
        );
        assert_eq!(payload, from_bson::<Payload>(bson).unwrap());
    }

    #[test]
    fn test_tagged_data_payload_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
        let payload = Payload::rand_tagged_data();
        iota::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(
            bson.as_document().unwrap().get_str("kind").unwrap(),
            TaggedDataPayload::KIND
        );
        assert_eq!(payload, from_bson::<Payload>(bson).unwrap());
    }
}
