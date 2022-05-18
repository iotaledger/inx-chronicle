// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_message_stardust::payload::transaction as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::message::{Input, Output, Payload, UnlockBlock};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransactionId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl TransactionId {
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<bee::TransactionId> for TransactionId {
    fn from(value: bee::TransactionId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<TransactionId> for bee::TransactionId {
    type Error = crate::types::error::Error;

    fn try_from(value: TransactionId) -> Result<Self, Self::Error> {
        Ok(bee::TransactionId::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for TransactionId {
    type Err = crate::types::error::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::TransactionId::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionPayload {
    pub id: TransactionId,
    pub essence: TransactionEssence,
    pub unlock_blocks: Box<[UnlockBlock]>,
}

impl From<&bee::TransactionPayload> for TransactionPayload {
    fn from(value: &bee::TransactionPayload) -> Self {
        Self {
            id: value.id().into(),
            essence: value.essence().into(),
            unlock_blocks: value.unlock_blocks().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<TransactionPayload> for bee::TransactionPayload {
    type Error = crate::types::error::Error;

    fn try_from(value: TransactionPayload) -> Result<Self, Self::Error> {
        Ok(bee::TransactionPayload::new(
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TransactionEssence {
    #[serde(rename = "regular")]
    Regular {
        #[serde(with = "crate::types::stringify")]
        network_id: u64,
        inputs: Box<[Input]>,
        #[serde(with = "serde_bytes")]
        inputs_commitment: Box<[u8]>,
        outputs: Box<[Output]>,
        payload: Option<Payload>,
    },
}

impl From<&bee::TransactionEssence> for TransactionEssence {
    fn from(value: &bee::TransactionEssence) -> Self {
        match value {
            bee::TransactionEssence::Regular(essence) => Self::Regular {
                network_id: essence.network_id(),
                inputs: essence.inputs().iter().map(Into::into).collect(),
                inputs_commitment: essence.inputs_commitment().to_vec().into_boxed_slice(),
                outputs: essence.outputs().iter().map(Into::into).collect(),
                payload: essence.payload().map(Into::into),
            },
        }
    }
}

impl TryFrom<TransactionEssence> for bee::TransactionEssence {
    type Error = crate::types::error::Error;

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
                let mut builder = bee::RegularTransactionEssence::builder(
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
                bee::TransactionEssence::Regular(builder.finish()?)
            }
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    pub(crate) const OUTPUT_ID1: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492a00";
    pub(crate) const OUTPUT_ID2: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492b00";
    pub(crate) const OUTPUT_ID3: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492c00";
    pub(crate) const OUTPUT_ID4: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492d00";

    use bee_message_stardust::unlock_block::UnlockBlocks;
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::{
        output::test::{get_test_alias_output, get_test_basic_output, get_test_foundry_output, get_test_nft_output},
        unlock_block::test::{
            get_test_alias_unlock_block, get_test_nft_unlock_block, get_test_reference_unlock_block,
            get_test_signature_unlock_block,
        },
        OutputId,
    };

    #[test]
    fn test_transaction_payload_bson() {
        let payload = get_test_transaction_payload();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<TransactionPayload>(bson).unwrap());
    }

    pub(crate) fn get_test_transaction_essence() -> TransactionEssence {
        TransactionEssence::from(&bee::TransactionEssence::Regular(
            bee::RegularTransactionEssenceBuilder::new(0, [0; 32].into())
                .with_inputs(vec![
                    Input::Utxo(OutputId::from_str(OUTPUT_ID1).unwrap()).try_into().unwrap(),
                    Input::Utxo(OutputId::from_str(OUTPUT_ID2).unwrap()).try_into().unwrap(),
                    Input::Utxo(OutputId::from_str(OUTPUT_ID3).unwrap()).try_into().unwrap(),
                    Input::Utxo(OutputId::from_str(OUTPUT_ID4).unwrap()).try_into().unwrap(),
                ])
                .with_outputs(vec![
                    get_test_basic_output().try_into().unwrap(),
                    get_test_alias_output().try_into().unwrap(),
                    get_test_foundry_output().try_into().unwrap(),
                    get_test_nft_output().try_into().unwrap(),
                ])
                .finish()
                .unwrap(),
        ))
    }

    pub(crate) fn get_test_transaction_payload() -> TransactionPayload {
        TransactionPayload::from(
            &bee::TransactionPayload::new(
                get_test_transaction_essence().try_into().unwrap(),
                UnlockBlocks::new(vec![
                    get_test_signature_unlock_block().try_into().unwrap(),
                    get_test_reference_unlock_block().try_into().unwrap(),
                    get_test_alias_unlock_block().try_into().unwrap(),
                    get_test_nft_unlock_block().try_into().unwrap(),
                ])
                .unwrap(),
            )
            .unwrap(),
        )
    }
}
