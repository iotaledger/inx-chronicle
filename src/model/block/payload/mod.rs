// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Payload`] types.

use std::borrow::Borrow;

use iota_sdk::types::block::payload::{self as iota};
use serde::{Deserialize, Serialize};

pub mod tagged_data;
pub mod transaction;

pub use self::{tagged_data::TaggedDataPayloadDto, transaction::SignedTransactionPayloadDto};

/// The different payloads of a [`Block`](crate::model::Block).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PayloadDto {
    /// Signals a transaction of tokens.
    SignedTransaction(Box<SignedTransactionPayloadDto>),
    /// Signals arbitrary data as a key-value pair.
    TaggedData(Box<TaggedDataPayloadDto>),
    /// A candidacy announcement payload.
    CandidacyAnnouncement,
}

impl<T: Borrow<iota::Payload>> From<T> for PayloadDto {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::Payload::SignedTransaction(p) => Self::SignedTransaction(Box::new(p.as_ref().into())),
            iota::Payload::TaggedData(p) => Self::TaggedData(Box::new(p.as_ref().into())),
            iota::Payload::CandidacyAnnouncement(_) => Self::CandidacyAnnouncement,
        }
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{doc, from_bson, to_bson, to_document};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_transaction_payload_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let payload = PayloadDto::rand_transaction(&ctx);
//         let mut bson = to_bson(&payload).unwrap();
//         // Need to re-add outputs as they are not serialized
//         let outputs_doc = if let PayloadDto::Transaction(payload) = &payload {
//             let TransactionEssence::Regular { outputs, .. } = &payload.essence;
//             doc! { "outputs": outputs.iter().map(to_document).collect::<Result<Vec<_>, _>>().unwrap() }
//         } else {
//             unreachable!();
//         };
//         let doc = bson.as_document_mut().unwrap().get_document_mut("essence").unwrap();
//         doc.extend(outputs_doc);
//         assert_eq!(
//             bson.as_document().unwrap().get_str("kind").unwrap(),
//             TransactionPayloadDto::KIND
//         );
//         assert_eq!(payload, from_bson::<PayloadDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_milestone_payload_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let payload = PayloadDto::rand_milestone(&ctx);
//         iota::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
//         let bson = to_bson(&payload).unwrap();
//         assert_eq!(
//             bson.as_document().unwrap().get_str("kind").unwrap(),
//             MilestonePayload::KIND
//         );
//         assert_eq!(payload, from_bson::<PayloadDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_treasury_transaction_payload_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let payload = PayloadDto::rand_treasury_transaction(&ctx);
//         iota::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
//         let bson = to_bson(&payload).unwrap();
//         assert_eq!(
//             bson.as_document().unwrap().get_str("kind").unwrap(),
//             TreasuryTransactionPayload::KIND
//         );
//         assert_eq!(payload, from_bson::<PayloadDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_tagged_data_payload_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let payload = PayloadDto::rand_tagged_data();
//         iota::Payload::try_from_with_context(&ctx, payload.clone()).unwrap();
//         let bson = to_bson(&payload).unwrap();
//         assert_eq!(
//             bson.as_document().unwrap().get_str("kind").unwrap(),
//             TaggedDataPayloadDto::KIND
//         );
//         assert_eq!(payload, from_bson::<PayloadDto>(bson).unwrap());
//     }
// }
