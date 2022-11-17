// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{blake2b::Blake2b256, Digest, Output};
use serde::{Deserialize, Serialize};

use super::merkle_hasher::MerkleHasher;

#[derive(Clone)]
pub struct MerkleProof<H: Default + Digest> {
    pub left: Hashable<H>,
    pub right: Hashable<H>,
}

impl<H: Default + Digest> PartialEq for MerkleProof<H> {
    fn eq(&self, other: &Self) -> bool {
        self.left == other.left && self.right == other.right
    }
}
impl<H: Default + Digest> Eq for MerkleProof<H> {}
impl<H: Default + Digest> std::fmt::Debug for MerkleProof<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MerkleProof")
            .field("left", &self.left)
            .field("right", &self.right)
            .finish()
    }
}

impl<H: Default + Digest> MerkleProof<H> {
    pub fn hash(&self) -> Output<H> {
        let l = self.left.hash();
        let r = self.right.hash();
        MerkleHasher::hash_node::<H>(l.as_ref(), r.as_ref())
    }

    pub fn contains_block_id(&self, block_id: &BlockId) -> bool {
        let value = MerkleHasher::hash_leaf::<H>(block_id.0);
        self.contains_value(&value)
    }

    pub fn contains_value(&self, value: &impl AsRef<[u8]>) -> bool {
        self.left.contains_value(value) || self.right.contains_value(value)
    }
}

#[derive(Clone)]
pub enum Hashable<H: Default + Digest> {
    MerkleProof(Box<MerkleProof<H>>),
    Node(Output<H>),
    Value(Output<H>),
}

impl<H: Default + Digest> PartialEq for Hashable<H> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MerkleProof(l0), Self::MerkleProof(r0)) => l0 == r0,
            (Self::Node(l0), Self::Node(r0)) => l0 == r0,
            (Self::Value(l0), Self::Value(r0)) => l0 == r0,
            _ => false,
        }
    }
}
impl<H: Default + Digest> Eq for Hashable<H> {}
impl<H: Default + Digest> std::fmt::Debug for Hashable<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MerkleProof(arg0) => f.debug_tuple("MerkleProof").field(arg0).finish(),
            Self::Node(arg0) => f.debug_tuple("Node").field(arg0).finish(),
            Self::Value(arg0) => f.debug_tuple("Value").field(arg0).finish(),
        }
    }
}

impl<H: Default + Digest> Hashable<H> {
    fn hash(&self) -> Cow<Output<H>> {
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

impl<H: Default + Digest> From<MerkleProof<H>> for MerkleProofDto {
    fn from(value: MerkleProof<H>) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}

impl TryFrom<MerkleProofDto> for MerkleProof<Blake2b256> {
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

impl<H: Default + Digest> From<Hashable<H>> for HashableDto {
    fn from(value: Hashable<H>) -> Self {
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

impl TryFrom<HashableDto> for Hashable<Blake2b256> {
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
