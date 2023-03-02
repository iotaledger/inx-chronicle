// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing types related to transactions.

use std::{borrow::Borrow, str::FromStr};

use iota_types::block::{output::InputsCommitment, payload::transaction as iota};
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use crate::model::{bytify, stringify, Input, Output, Payload, TryFromWithContext, TryIntoWithContext, Unlock};

/// Uniquely identifies a transaction.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct TransactionId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl TransactionId {
    /// The number of bytes for the id.
    pub const LENGTH: usize = iota::TransactionId::LENGTH;

    /// Converts the [`TransactionId`] to its `0x`-prefixed hex representation.
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<iota::TransactionId> for TransactionId {
    fn from(value: iota::TransactionId) -> Self {
        Self(*value)
    }
}

impl From<TransactionId> for iota::TransactionId {
    fn from(value: TransactionId) -> Self {
        iota::TransactionId::new(value.0)
    }
}

impl FromStr for TransactionId {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::TransactionId::from_str(s)?.into())
    }
}

impl From<TransactionId> for Bson {
    fn from(val: TransactionId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

/// Represents the transaction payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionPayload {
    /// The id of the transaction.
    pub transaction_id: TransactionId,
    /// The transaction essence.
    pub essence: TransactionEssence,
    /// The list of unlocks.
    pub unlocks: Box<[Unlock]>,
}

impl TransactionPayload {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "transaction";
}

impl<T: Borrow<iota::TransactionPayload>> From<T> for TransactionPayload {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            transaction_id: value.id().into(),
            essence: value.essence().into(),
            unlocks: value.unlocks().iter().map(Into::into).collect(),
        }
    }
}

impl TryFromWithContext<TransactionPayload> for iota::TransactionPayload {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: TransactionPayload,
    ) -> Result<Self, Self::Error> {
        iota::TransactionPayload::new(
            value.essence.try_into_with_context(ctx)?,
            iota_types::block::unlock::Unlocks::new(
                value
                    .unlocks
                    .into_vec()
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        )
    }
}

impl From<TransactionPayload> for iota::dto::TransactionPayloadDto {
    fn from(value: TransactionPayload) -> Self {
        Self {
            kind: iota::TransactionPayload::KIND,
            essence: value.essence.into(),
            unlocks: value.unlocks.into_vec().into_iter().map(Into::into).collect(),
        }
    }
}

/// Represents the essence of a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TransactionEssence {
    /// The regular transaction essence.
    Regular {
        /// The network id for which this transaction was issued.
        /// Note: Including the network id in the transaction prevents replay attacks.
        #[serde(with = "stringify")]
        network_id: u64,
        /// The list of inputs that this transaction consumes.
        inputs: Box<[Input]>,
        #[serde(with = "bytify")]
        /// The input commitment hash as bytes.
        inputs_commitment: [u8; Self::INPUTS_COMMITMENT_LENGTH],
        /// The list of outputs that this transaction creates.
        #[serde(skip_serializing)]
        outputs: Box<[Output]>,
        /// The [`Payload`], which for now can only be of type [`TaggedDataPayload`](super::TaggedDataPayload).
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Payload>,
    },
}

impl TransactionEssence {
    const INPUTS_COMMITMENT_LENGTH: usize = InputsCommitment::LENGTH;
}

impl<T: Borrow<iota::TransactionEssence>> From<T> for TransactionEssence {
    fn from(value: T) -> Self {
        let value = value.borrow();
        match value {
            iota::TransactionEssence::Regular(essence) => Self::Regular {
                network_id: essence.network_id(),
                inputs: essence.inputs().iter().map(Into::into).collect(),
                inputs_commitment: **essence.inputs_commitment(),
                outputs: essence.outputs().iter().map(Into::into).collect(),
                payload: essence.payload().map(Into::into),
            },
        }
    }
}

impl TryFromWithContext<TransactionEssence> for iota::TransactionEssence {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: TransactionEssence,
    ) -> Result<Self, Self::Error> {
        Ok(match value {
            TransactionEssence::Regular {
                network_id: _,
                inputs,
                inputs_commitment,
                outputs,
                payload,
            } => {
                let mut builder = iota::RegularTransactionEssence::builder(
                    ctx.network_id(),
                    iota_types::block::output::InputsCommitment::from(inputs_commitment),
                )
                .with_inputs(
                    inputs
                        .into_vec()
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .with_outputs(
                    outputs
                        .into_vec()
                        .into_iter()
                        .map(|x| x.try_into_with_context(ctx))
                        .collect::<Result<Vec<_>, _>>()?,
                );
                if let Some(payload) = payload {
                    builder = builder.with_payload(payload.try_into_with_context(ctx)?);
                }
                iota::TransactionEssence::Regular(builder.finish(ctx)?)
            }
        })
    }
}

impl From<TransactionEssence> for iota::dto::TransactionEssenceDto {
    fn from(value: TransactionEssence) -> Self {
        match value {
            TransactionEssence::Regular {
                network_id,
                inputs,
                inputs_commitment,
                outputs,
                payload,
            } => Self::Regular(iota::dto::RegularTransactionEssenceDto {
                kind: iota::RegularTransactionEssence::KIND,
                network_id: network_id.to_string(),
                inputs: inputs.into_vec().into_iter().map(Into::into).collect(),
                inputs_commitment: prefix_hex::encode(inputs_commitment),
                outputs: outputs.into_vec().into_iter().map(Into::into).collect(),
                payload: payload.map(Into::into),
            }),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::{
        bytes::rand_bytes_array,
        number::{rand_number, rand_number_range},
        output::rand_inputs_commitment,
    };

    use super::*;

    impl TransactionId {
        /// Generates a random [`TransactionId`].
        pub fn rand() -> Self {
            Self(rand_bytes_array())
        }
    }

    impl TransactionEssence {
        /// Generates a random [`TransactionEssence`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self::Regular {
                network_id: rand_number(),
                inputs: std::iter::repeat_with(Input::rand)
                    .take(rand_number_range(0..10))
                    .collect(),
                inputs_commitment: *rand_inputs_commitment(),
                outputs: std::iter::repeat_with(|| Output::rand(ctx))
                    .take(rand_number_range(0..10))
                    .collect(),
                payload: if rand_number_range(0..=1) == 1 {
                    Some(Payload::rand_tagged_data())
                } else {
                    None
                },
            }
        }
    }

    impl TransactionPayload {
        /// Generates a random [`TransactionPayload`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self {
                transaction_id: TransactionId::rand(),
                essence: TransactionEssence::rand(ctx),
                unlocks: std::iter::repeat_with(Unlock::rand)
                    .take(rand_number_range(1..10))
                    .collect(),
            }
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{doc, from_bson, to_bson, to_document};

    use super::*;

    #[test]
    fn test_transaction_id_bson() {
        let transaction_id = TransactionId::rand();
        let bson = to_bson(&transaction_id).unwrap();
        assert_eq!(Bson::from(transaction_id), bson);
        assert_eq!(transaction_id, from_bson::<TransactionId>(bson).unwrap());
    }

    #[test]
    fn test_transaction_payload_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
        let payload = TransactionPayload::rand(&ctx);
        let mut bson = to_bson(&payload).unwrap();
        // Need to re-add outputs as they are not serialized
        let TransactionEssence::Regular { outputs, .. } = &payload.essence;
        let outputs_doc = doc! { "outputs": outputs.iter().map(to_document).collect::<Result<Vec<_>, _>>().unwrap() };
        let doc = bson.as_document_mut().unwrap().get_document_mut("essence").unwrap();
        doc.extend(outputs_doc);
        assert_eq!(payload, from_bson::<TransactionPayload>(bson).unwrap());
    }
}
