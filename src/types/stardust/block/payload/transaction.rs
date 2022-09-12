// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::{output::InputsCommitment, payload::transaction as bee};
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use crate::types::{
    stardust::block::{Input, Output, Payload, Unlock},
    util::bytify,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransactionId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl TransactionId {
    const LENGTH: usize = bee::TransactionId::LENGTH;

    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<bee::TransactionId> for TransactionId {
    fn from(value: bee::TransactionId) -> Self {
        Self(*value)
    }
}

impl From<TransactionId> for bee::TransactionId {
    fn from(value: TransactionId) -> Self {
        bee::TransactionId::new(value.0)
    }
}

impl FromStr for TransactionId {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::TransactionId::from_str(s)?.into())
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionPayload {
    pub transaction_id: TransactionId,
    pub essence: TransactionEssence,
    pub unlocks: Box<[Unlock]>,
}

impl From<&bee::TransactionPayload> for TransactionPayload {
    fn from(value: &bee::TransactionPayload) -> Self {
        Self {
            transaction_id: value.id().into(),
            essence: value.essence().into(),
            unlocks: value.unlocks().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<TransactionPayload> for bee::TransactionPayload {
    type Error = bee_block_stardust::Error;

    fn try_from(value: TransactionPayload) -> Result<Self, Self::Error> {
        bee::TransactionPayload::new(
            value.essence.try_into()?,
            bee_block_stardust::unlock::Unlocks::new(
                Vec::from(value.unlocks)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TransactionEssence {
    Regular {
        #[serde(with = "crate::types::util::stringify")]
        network_id: u64,
        inputs: Box<[Input]>,
        #[serde(with = "bytify")]
        inputs_commitment: [u8; Self::INPUTS_COMMITMENT_LENGTH],
        outputs: Box<[Output]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Payload>,
    },
}

impl TransactionEssence {
    const INPUTS_COMMITMENT_LENGTH: usize = InputsCommitment::LENGTH;
}

impl From<&bee::TransactionEssence> for TransactionEssence {
    fn from(value: &bee::TransactionEssence) -> Self {
        match value {
            bee::TransactionEssence::Regular(essence) => Self::Regular {
                network_id: essence.network_id(),
                inputs: essence.inputs().iter().map(Into::into).collect(),
                inputs_commitment: **essence.inputs_commitment(),
                outputs: essence.outputs().iter().map(Into::into).collect(),
                payload: essence.payload().map(Into::into),
            },
        }
    }
}

impl TryFrom<TransactionEssence> for bee::TransactionEssence {
    type Error = bee_block_stardust::Error;

    fn try_from(value: TransactionEssence) -> Result<Self, Self::Error> {
        Ok(match value {
            TransactionEssence::Regular {
                network_id,
                inputs,
                inputs_commitment,
                outputs,
                payload,
            } => {
                let mut builder = bee::RegularTransactionEssence::builder(
                    network_id,
                    bee_block_stardust::output::InputsCommitment::from(inputs_commitment),
                )
                .with_inputs(
                    Vec::from(inputs)
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .with_outputs(
                    Vec::from(outputs)
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                );
                if let Some(payload) = payload {
                    builder = builder.with_payload(payload.try_into()?);
                }
                bee::TransactionEssence::Regular(builder.finish()?)
            }
        })
    }
}

#[cfg(test)]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::util::payload::transaction::get_test_transaction_payload;

    #[test]
    fn test_transaction_id_bson() {
        let transaction_id = TransactionId::from(bee_block_stardust::rand::transaction::rand_transaction_id());
        let bson = to_bson(&transaction_id).unwrap();
        assert_eq!(Bson::from(transaction_id), bson);
        assert_eq!(transaction_id, from_bson::<TransactionId>(bson).unwrap());
    }

    #[test]
    fn test_transaction_payload_bson() {
        let payload = get_test_transaction_payload();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<TransactionPayload>(bson).unwrap());
    }
}
