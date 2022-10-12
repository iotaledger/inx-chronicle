// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod address;
pub mod block_id;
pub mod input;
pub mod output;
pub mod payload;
pub mod signature;
pub mod unlock;

use bee::protocol::ProtocolParameters;
use bee_block_stardust as bee;
use serde::{Deserialize, Serialize};

pub use self::{
    address::Address, block_id::BlockId, input::Input, output::Output, payload::Payload, signature::Signature,
    unlock::Unlock,
};
use crate::types::context::{TryFromWithContext, TryIntoWithContext};

/// The Block type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// The protocol version from when the block was issued.
    pub protocol_version: u8,
    /// The parents of the block.
    pub parents: Box<[BlockId]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The payload of the block.
    pub payload: Option<Payload>,
    /// The nonce determined by proof-of-work.
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

impl TryFromWithContext<Block> for bee::Block {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(ctx: &ProtocolParameters, value: Block) -> Result<Self, Self::Error> {
        let mut builder = bee::BlockBuilder::<u64>::new(bee::parent::Parents::new(
            value.parents.into_vec().into_iter().map(Into::into).collect::<Vec<_>>(),
        )?)
        .with_nonce_provider(value.nonce);
        if let Some(payload) = value.payload {
            builder = builder.with_payload(payload.try_into_with_context(ctx)?)
        }
        builder.finish(ctx.min_pow_score())
    }
}

impl TryFromWithContext<Block> for bee::BlockDto {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(ctx: &ProtocolParameters, value: Block) -> Result<Self, Self::Error> {
        let stardust = bee::Block::try_from_with_context(ctx, value)?;
        Ok(Self::from(&stardust))
    }
}

impl From<Block> for bee::BlockDto {
    fn from(value: Block) -> Self {
        Self {
            protocol_version: value.protocol_version,
            parents: value.parents.to_vec().iter().map(BlockId::to_hex).collect(),
            payload: value.payload.map(Into::into),
            nonce: value.nonce.to_string(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee::rand::number::rand_number;

    use super::*;

    impl Block {
        /// Generates a random [`Block`].
        pub fn rand(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Payload::rand_opt(ctx),
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with a [`TransactionPayload`](payload::TransactionPayload).
        pub fn rand_transaction(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Some(Payload::rand_transaction(ctx)),
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with a spending [`TransactionPayload`](payload::TransactionPayload).
        pub fn rand_spending_transaction(
            ctx: &bee_block_stardust::protocol::ProtocolParameters,
        ) -> (Self, Vec<Input>, Vec<Output>) {
            let (payload, inputs, outputs) = Payload::rand_spending_transaction(ctx);
            (
                Self {
                    protocol_version: rand_number(),
                    parents: BlockId::rand_parents(),
                    payload: Some(payload),
                    nonce: rand_number(),
                },
                inputs,
                outputs,
            )
        }

        /// Generates a random [`Block`] with a [`MilestonePayload`](payload::MilestonePayload).
        pub fn rand_milestone(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Some(Payload::rand_milestone(ctx)),
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with a [`TaggedDataPayload`](payload::TaggedDataPayload).
        pub fn rand_tagged_data() -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Some(Payload::rand_tagged_data()),
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with a
        /// [`TreasuryTransactionPayload`](payload::TreasuryTransactionPayload).
        pub fn rand_treasury_transaction(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Some(Payload::rand_treasury_transaction(ctx)),
                nonce: rand_number(),
            }
        }
        /// Generates a random [`Block`] with no payload.
        pub fn rand_no_payload() -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: None,
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with given parents.
        pub fn rand_no_payload_with_parents(parents: Box<[BlockId]>) -> Self {
            Self {
                protocol_version: rand_number(),
                parents,
                payload: None,
                nonce: rand_number(),
            }
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{doc, from_bson, to_bson, to_document, Bson};

    use super::*;
    use crate::types::stardust::block::payload::TransactionEssence;

    #[test]
    fn test_block_id_bson() {
        let block_id = BlockId::rand();
        let bson = to_bson(&block_id).unwrap();
        assert_eq!(Bson::from(block_id), bson);
        from_bson::<BlockId>(bson).unwrap();
    }

    #[test]
    fn test_transaction_block_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let block = Block::rand_transaction(&ctx);
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
    }

    #[test]
    fn test_milestone_block_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let block = Block::rand_milestone(&ctx);
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        bee::Block::try_from_with_context(&ctx, block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }

    #[test]
    fn test_tagged_data_block_bson() {
        let block = Block::rand_tagged_data();
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        bee::Block::try_from_with_context(&ctx, block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }

    #[test]
    fn test_treasury_transaction_block_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let block = Block::rand_treasury_transaction(&ctx);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }

    #[test]
    fn test_no_payload_block_bson() {
        let block = Block::rand_no_payload();
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        bee::Block::try_from_with_context(&ctx, block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }
}
