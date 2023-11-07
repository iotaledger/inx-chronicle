// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing types related to transactions.

use std::borrow::Borrow;

use iota_sdk::{
    types::block::{
        context_input::ContextInput,
        mana::ManaAllotment,
        output::AccountId,
        payload::{
            signed_transaction::{self as iota, TransactionCapabilities},
            Payload,
        },
        slot::SlotIndex,
    },
    utils::serde::string,
};
use serde::{Deserialize, Serialize};

use self::{input::InputDto, output::OutputDto, unlock::UnlockDto};
use super::TaggedDataPayloadDto;

pub mod input;
pub mod output;
pub mod unlock;

/// Represents the transaction payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedTransactionPayloadDto {
    pub transaction_id: iota::TransactionId,
    pub transaction: TransactionDto,
    pub unlocks: Vec<UnlockDto>,
}

impl SignedTransactionPayloadDto {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "transaction";
}

impl<T: Borrow<iota::SignedTransactionPayload>> From<T> for SignedTransactionPayloadDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            transaction_id: value.transaction().id().into(),
            transaction: value.transaction().into(),
            unlocks: value.unlocks().iter().map(Into::into).collect(),
        }
    }
}

/// Represents the essence of a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionDto {
    network_id: u64,
    creation_slot: SlotIndex,
    context_inputs: Vec<ContextInput>,
    inputs: Vec<InputDto>,
    mana_allotments: Vec<ManaAllotmentDto>,
    capabilities: TransactionCapabilities,
    payload: Option<TaggedDataPayloadDto>,
    #[serde(skip_serializing)]
    outputs: Vec<OutputDto>,
}

impl<T: Borrow<iota::Transaction>> From<T> for TransactionDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            network_id: value.network_id(),
            creation_slot: value.creation_slot(),
            context_inputs: value.context_inputs().iter().cloned().collect(),
            inputs: value.inputs().iter().map(Into::into).collect(),
            mana_allotments: value.mana_allotments().iter().map(Into::into).collect(),
            capabilities: value.capabilities().clone(),
            payload: value.payload().map(Payload::as_tagged_data).map(Into::into),
            outputs: value.outputs().iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaAllotmentDto {
    pub account_id: AccountId,
    #[serde(with = "string")]
    pub mana: u64,
}

impl<T: Borrow<ManaAllotment>> From<T> for ManaAllotmentDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            account_id: *value.account_id(),
            mana: value.mana(),
        }
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{doc, from_bson, to_bson, to_document};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_transaction_id_bson() {
//         let transaction_id = TransactionId::rand();
//         let bson = to_bson(&transaction_id).unwrap();
//         assert_eq!(Bson::from(transaction_id), bson);
//         assert_eq!(transaction_id, from_bson::<TransactionId>(bson).unwrap());
//     }

//     #[test]
//     fn test_transaction_payload_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let payload = TransactionPayloadDto::rand(&ctx);
//         let mut bson = to_bson(&payload).unwrap();
//         // Need to re-add outputs as they are not serialized
//         let TransactionEssence::Regular { outputs, .. } = &payload.essence;
//         let outputs_doc = doc! { "outputs": outputs.iter().map(to_document).collect::<Result<Vec<_>, _>>().unwrap()
// };         let doc = bson.as_document_mut().unwrap().get_document_mut("essence").unwrap();
//         doc.extend(outputs_doc);
//         assert_eq!(payload, from_bson::<TransactionPayloadDto>(bson).unwrap());
//     }
// }
