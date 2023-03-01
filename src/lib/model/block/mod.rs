// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod block_metadata;
pub mod payload;
pub mod signature;

use std::str::FromStr;

use iota::protocol::ProtocolParameters;
use iota_types::block as iota;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

pub use self::block_metadata::BlockMetadata;
use super::Payload;
use crate::model::{
    serde::{bytify, stringify},
    tangle::{TryFromWithContext, TryIntoWithContext},
};

/// Uniquely identifies a block.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Ord, PartialOrd, Eq)]
#[serde(transparent)]
pub struct BlockId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl BlockId {
    /// The number of bytes for the id.
    pub const LENGTH: usize = iota::BlockId::LENGTH;

    /// The `0x`-prefixed hex representation of a [`BlockId`].
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<iota::BlockId> for BlockId {
    fn from(value: iota::BlockId) -> Self {
        Self(*value)
    }
}

impl From<BlockId> for iota::BlockId {
    fn from(value: BlockId) -> Self {
        iota::BlockId::new(value.0)
    }
}

impl FromStr for BlockId {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::BlockId::from_str(s)?.into())
    }
}

impl From<BlockId> for Bson {
    fn from(val: BlockId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

impl AsRef<[u8]> for BlockId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

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
    #[serde(with = "stringify")]
    pub nonce: u64,
}

impl From<iota::Block> for Block {
    fn from(value: iota::Block) -> Self {
        Self {
            protocol_version: value.protocol_version(),
            parents: value.parents().iter().map(|&id| BlockId::from(id)).collect(),
            payload: value.payload().map(Into::into),
            nonce: value.nonce(),
        }
    }
}

impl TryFromWithContext<Block> for iota::Block {
    type Error = iota_types::block::Error;

    fn try_from_with_context(ctx: &ProtocolParameters, value: Block) -> Result<Self, Self::Error> {
        let mut builder = iota::BlockBuilder::new(iota::parent::Parents::new(
            value.parents.into_vec().into_iter().map(Into::into).collect::<Vec<_>>(),
        )?)
        .with_nonce(value.nonce);
        if let Some(payload) = value.payload {
            builder = builder.with_payload(payload.try_into_with_context(ctx)?)
        }
        builder.finish()
    }
}

impl TryFromWithContext<Block> for iota::BlockDto {
    type Error = iota_types::block::Error;

    fn try_from_with_context(ctx: &ProtocolParameters, value: Block) -> Result<Self, Self::Error> {
        let stardust = iota::Block::try_from_with_context(ctx, value)?;
        Ok(Self::from(&stardust))
    }
}

impl From<Block> for iota::BlockDto {
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
    use iota::rand::{
        block::{rand_block_id, rand_block_ids},
        number::rand_number,
    };

    use super::*;

    impl BlockId {
        /// Generates a random [`BlockId`].
        pub fn rand() -> Self {
            rand_block_id().into()
        }

        /// Generates multiple random [`BlockIds`](BlockId).
        pub fn rand_many(len: usize) -> impl Iterator<Item = Self> {
            rand_block_ids(len).into_iter().map(Into::into)
        }

        /// Generates a random amount of parents.
        pub fn rand_parents() -> Box<[Self]> {
            Self::rand_many(*iota::parent::Parents::COUNT_RANGE.end() as _).collect()
        }
    }

    impl Block {
        /// Generates a random [`Block`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Payload::rand_opt(ctx),
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with a [`TransactionPayload`](crate::model::payload::TransactionPayload).
        pub fn rand_transaction(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Some(Payload::rand_transaction(ctx)),
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with a [`MilestonePayload`](crate::model::payload::MilestonePayload).
        pub fn rand_milestone(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Some(Payload::rand_milestone(ctx)),
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with a [`TaggedDataPayload`](crate::model::payload::TaggedDataPayload).
        pub fn rand_tagged_data() -> Self {
            Self {
                protocol_version: rand_number(),
                parents: BlockId::rand_parents(),
                payload: Some(Payload::rand_tagged_data()),
                nonce: rand_number(),
            }
        }

        /// Generates a random [`Block`] with a
        /// [`TreasuryTransactionPayload`](crate::model::payload::TreasuryTransactionPayload).
        pub fn rand_treasury_transaction(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
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
    use crate::model::payload::TransactionEssence;

    #[test]
    fn test_block_id_bson() {
        let block_id = BlockId::rand();
        let bson = to_bson(&block_id).unwrap();
        assert_eq!(Bson::from(block_id), bson);
        from_bson::<BlockId>(bson).unwrap();
    }

    #[test]
    fn test_transaction_block_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
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
        let ctx = iota_types::block::protocol::protocol_parameters();
        let block = Block::rand_milestone(&ctx);
        let ctx = iota_types::block::protocol::protocol_parameters();
        iota::Block::try_from_with_context(&ctx, block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }

    #[test]
    fn test_tagged_data_block_bson() {
        let block = Block::rand_tagged_data();
        let ctx = iota_types::block::protocol::protocol_parameters();
        iota::Block::try_from_with_context(&ctx, block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }

    #[test]
    fn test_treasury_transaction_block_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
        let block = Block::rand_treasury_transaction(&ctx);
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }

    #[test]
    fn test_no_payload_block_bson() {
        let block = Block::rand_no_payload();
        let ctx = iota_types::block::protocol::protocol_parameters();
        iota::Block::try_from_with_context(&ctx, block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Block>(bson).unwrap());
    }
}
