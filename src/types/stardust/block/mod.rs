// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod address;
mod input;
mod output;
mod payload;
mod signature;
mod unlock;

use bee_block_stardust as stardust;
use serde::{Deserialize, Serialize};

pub use self::{address::*, input::*, output::*, payload::*, signature::*, unlock::*};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Ord, PartialOrd, Eq)]
#[serde(transparent)]
pub struct BlockId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl BlockId {
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<stardust::BlockId> for BlockId {
    fn from(value: stardust::BlockId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<BlockId> for stardust::BlockId {
    type Error = crate::types::error::Error;

    fn try_from(value: BlockId) -> Result<Self, Self::Error> {
        Ok(stardust::BlockId::new(value.0.as_ref().try_into()?))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub protocol_version: u8,
    pub parents: Box<[BlockId]>,
    pub payload: Option<Payload>,
    #[serde(with = "crate::types::stringify")]
    pub nonce: u64,
}

impl From<stardust::Block> for Block {
    fn from(value: stardust::Block) -> Self {
        Self {
            id: value.id().into(),
            protocol_version: value.protocol_version(),
            parents: value.parents().iter().map(|id| BlockId::from(*id)).collect(),
            payload: value.payload().map(Into::into),
            nonce: value.nonce(),
        }
    }
}

impl TryFrom<Block> for stardust::Block {
    type Error = crate::types::error::Error;

    fn try_from(value: Block) -> Result<Self, Self::Error> {
        let mut builder = stardust::BlockBuilder::<u64>::new(stardust::parent::Parents::new(
            Vec::from(value.parents)
                .into_iter()
                .map(|p| p.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        )?)
        .with_nonce_provider(value.nonce, 0.0);
        if let Some(payload) = value.payload {
            builder = builder.with_payload(payload.try_into()?)
        }
        Ok(builder.finish()?)
    }
}
