// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::payload::transaction as stardust;
use serde::{Deserialize, Serialize};

use crate::dto;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransactionId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<stardust::TransactionId> for TransactionId {
    fn from(value: stardust::TransactionId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<TransactionId> for stardust::TransactionId {
    type Error = crate::dto::error::Error;

    fn try_from(value: TransactionId) -> Result<Self, Self::Error> {
        Ok(stardust::TransactionId::new(value.0.as_ref().try_into()?))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransactionPayload {
    pub id: TransactionId,
    pub essence: TransactionEssence,
    pub unlock_blocks: Box<[dto::UnlockBlock]>,
}

impl From<&stardust::TransactionPayload> for TransactionPayload {
    fn from(value: &stardust::TransactionPayload) -> Self {
        Self {
            id: value.id().into(),
            essence: value.essence().into(),
            unlock_blocks: value.unlock_blocks().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<TransactionPayload> for stardust::TransactionPayload {
    type Error = crate::dto::error::Error;

    fn try_from(value: TransactionPayload) -> Result<Self, Self::Error> {
        Ok(stardust::TransactionPayload::new(
            value.essence.try_into()?,
            bee_message_stardust::unlock_block::UnlockBlocks::new(
                Vec::from(value.unlock_blocks)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        )?)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TransactionEssence {
    #[serde(rename = "regular")]
    Regular {
        #[serde(with = "crate::dto::display_fromstr")]
        network_id: u64,
        inputs: Box<[dto::Input]>,
        #[serde(with = "serde_bytes")]
        inputs_commitment: Box<[u8]>,
        outputs: Box<[dto::Output]>,
        payload: Option<dto::Payload>,
    },
}

impl From<&stardust::TransactionEssence> for TransactionEssence {
    fn from(value: &stardust::TransactionEssence) -> Self {
        match value {
            stardust::TransactionEssence::Regular(essence) => Self::Regular {
                network_id: essence.network_id(),
                inputs: essence.inputs().iter().map(Into::into).collect(),
                inputs_commitment: essence.inputs_commitment().to_vec().into_boxed_slice(),
                outputs: essence.outputs().iter().map(Into::into).collect(),
                payload: essence.payload().map(Into::into),
            },
        }
    }
}

impl TryFrom<TransactionEssence> for stardust::TransactionEssence {
    type Error = crate::dto::error::Error;

    fn try_from(value: TransactionEssence) -> Result<Self, Self::Error> {
        Ok(match value {
            TransactionEssence::Regular {
                network_id,
                inputs,
                inputs_commitment: _,
                outputs,
                payload,
            } => {
                let outputs = Vec::from(outputs)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<bee_message_stardust::output::Output>, _>>()?;
                let mut builder = stardust::RegularTransactionEssence::builder(
                    network_id,
                    bee_message_stardust::output::InputsCommitment::new(outputs.iter()),
                )
                .with_inputs(
                    Vec::from(inputs)
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .with_outputs(outputs);
                if let Some(payload) = payload {
                    builder = builder.with_payload(payload.try_into()?);
                }
                stardust::TransactionEssence::Regular(builder.finish()?)
            }
        })
    }
}
