// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod address;
pub mod block_id;
pub mod input;
pub mod output;
pub mod payload;
pub mod signature;
pub mod unlock;

use bee_block_stardust as bee;
use serde::{Deserialize, Serialize};

pub use self::{
    address::Address, block_id::BlockId, input::Input, output::Output, payload::Payload, signature::Signature,
    unlock::Unlock,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub protocol_version: u8,
    pub parents: Box<[BlockId]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Payload>,
    #[serde(with = "crate::types::util::stringify")]
    pub nonce: u64,
}

impl From<bee::Block> for Block {
    fn from(value: bee::Block) -> Self {
        Self {
            protocol_version: value.protocol_version(),
            parents: value.parents().iter().map(|&id| BlockId::from(id)).collect(),
            payload: value.payload().map(Into::into),
            nonce: value.nonce(),
        }
    }
}

impl TryFrom<Block> for bee::Block {
    type Error = bee_block_stardust::Error;

    fn try_from(value: Block) -> Result<Self, Self::Error> {
        let mut builder = bee::BlockBuilder::<u64>::new(bee::parent::Parents::new(
            Vec::from(value.parents).into_iter().map(Into::into).collect::<Vec<_>>(),
        )?)
        .with_nonce_provider(value.nonce, 0);
        if let Some(payload) = value.payload {
            builder = builder.with_payload(payload.try_into()?)
        }
        builder.finish()
    }
}

impl TryFrom<Block> for bee::BlockDto {
    type Error = bee_block_stardust::Error;

    fn try_from(value: Block) -> Result<Self, Self::Error> {
        let stardust = bee::Block::try_from(value)?;
        Ok(Self::from(&stardust))
    }
}

#[cfg(test)]

mod test {
    use mongodb::bson::{doc, from_bson, to_bson, to_document, Bson};

    use super::*;
    use crate::types::stardust::{block::payload::TransactionEssence, util::*};

    #[test]
    fn test_block_id_bson() {
        let block_id = BlockId::from(bee_block_stardust::rand::block::rand_block_id());
        let bson = to_bson(&block_id).unwrap();
        assert_eq!(Bson::from(block_id), bson);
        from_bson::<BlockId>(bson).unwrap();
    }

    #[test]
    fn test_block_bson() {
        let block = get_test_transaction_block();
        let mut bson = to_bson(&block).unwrap();
        // Need to re-add outputs as they are not serialized
        let outputs_doc = if let Some(Payload::Transaction(payload)) = &block.payload {
            let TransactionEssence::Regular { outputs, .. } = &payload.essence;
            doc! { "outputs": outputs.iter().map(to_document).collect::<Result<Vec<_>, _>>().unwrap() }
        } else {
            unreachable!();
        };
        let doc = bson
            .as_document_mut()
            .unwrap()
            .get_document_mut("payload")
            .unwrap()
            .get_document_mut("essence")
            .unwrap();
        doc.extend(outputs_doc);
        assert_eq!(block, from_bson::<Block>(bson).unwrap());

        let block = get_test_milestone_block();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());

        let block = get_test_tagged_data_block();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }
}
