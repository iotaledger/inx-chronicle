// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{blake2b::Blake2b256, Output};
use serde::{Deserialize, Serialize};

use super::merkle_hasher::MerkleHasher;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleProof {
    pub left: Hashable,
    pub right: Hashable,
}

impl MerkleProof {
    pub fn hash(&self) -> Output<Blake2b256> {
        let l = self.left.hash();
        let r = self.right.hash();
        MerkleHasher::hash_node(l.as_ref(), r.as_ref())
    }

    pub fn contains_block_id(&self, block_id: &BlockId) -> bool {
        let value = MerkleHasher::hash_leaf(block_id.0);
        self.contains_value(&value)
    }

    pub fn contains_value(&self, value: &impl AsRef<[u8]>) -> bool {
        self.left.contains_value(value) || self.right.contains_value(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hashable {
    MerkleProof(Box<MerkleProof>),
    Node(Output<Blake2b256>),
    Value(Output<Blake2b256>),
}

impl Hashable {
    fn hash(&self) -> Cow<Output<Blake2b256>> {
        match self {
            Hashable::MerkleProof(proof) => Cow::Owned(proof.hash()),
            Hashable::Node(hash) | Hashable::Value(hash) => Cow::Borrowed(hash),
        }
    }

    fn contains_value(&self, value: &impl AsRef<[u8]>) -> bool {
        match self {
            Hashable::MerkleProof(p) => p.contains_value(value),
            Hashable::Node(_) => false,
            Hashable::Value(v) => &**v == value.as_ref(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleProofDto {
    #[serde(rename = "l")]
    left: HashableDto,
    #[serde(rename = "r")]
    right: HashableDto,
}

impl From<MerkleProof> for MerkleProofDto {
    fn from(value: MerkleProof) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}

impl TryFrom<MerkleProofDto> for MerkleProof {
    type Error = prefix_hex::Error;

    fn try_from(proof: MerkleProofDto) -> Result<Self, Self::Error> {
        Ok(Self {
            left: Hashable::try_from(proof.left)?,
            right: Hashable::try_from(proof.right)?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HashableDto {
    Node {
        #[serde(rename = "h")]
        hash: String,
    },
    Proof(Box<MerkleProofDto>),
    Value {
        #[serde(rename = "value")]
        value_hash: String,
    },
}

impl From<Hashable> for HashableDto {
    fn from(value: Hashable) -> Self {
        match value {
            Hashable::Node(h) => Self::Node {
                hash: prefix_hex::encode(h.as_slice()),
            },
            Hashable::Value(v) => Self::Value {
                value_hash: prefix_hex::encode(v.as_slice()),
            },
            Hashable::MerkleProof(p) => Self::Proof(Box::new((*p).into())),
        }
    }
}

impl TryFrom<HashableDto> for Hashable {
    type Error = prefix_hex::Error;

    fn try_from(hashed: HashableDto) -> Result<Self, Self::Error> {
        Ok(match hashed {
            HashableDto::Node { hash } => {
                let hash = prefix_hex::decode::<[u8; 32]>(&hash)?;
                Hashable::Node(hash.into())
            }
            HashableDto::Value { value_hash } => {
                let hash = prefix_hex::decode::<[u8; 32]>(&value_hash)?;
                Hashable::Value(hash.into())
            }
            HashableDto::Proof(proof) => {
                let proof = MerkleProof::try_from(*proof)?;
                Hashable::MerkleProof(Box::new(proof))
            }
        })
    }
}
