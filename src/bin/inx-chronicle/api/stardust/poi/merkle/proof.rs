// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{blake2b::Blake2b256, Output};
use serde::{Deserialize, Serialize};

use super::{error::CreateProofError, largest_power_of_two, MerkleHasher};

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

impl MerkleHasher {
    /// Create a merkle proof given a list of block IDs and a chosen block ID. The chosen leaf will become a
    /// value node, and the path will contain all hashes above it. The remaining branches will be terminated early.
    pub fn create_proof(block_ids: &[BlockId], chosen_block_id: &BlockId) -> Result<MerkleProof, CreateProofError> {
        let index = block_ids
            .iter()
            .position(|id| id == chosen_block_id)
            .ok_or_else(|| CreateProofError::BlockNotIncluded(chosen_block_id.to_hex()))?;
        Self::create_proof_from_index(block_ids, index)
    }

    // NOTE:
    // * `block_ids` is the list of past-cone block ids in "White Flag" order;
    // * `block_ids.len() >= 2` must be true, or this function panics;
    // * `index < block_ids.len()` must be true, or this function panics;
    fn create_proof_from_index(block_ids: &[BlockId], index: usize) -> Result<MerkleProof, CreateProofError> {
        if block_ids.len() < 2 {
            Err(CreateProofError::InsufficientBlockIds(block_ids.len()))
        } else {
            let n = block_ids.len();
            debug_assert!(index < n);

            let data = block_ids.iter().map(|block_id| block_id.0).collect::<Vec<_>>();
            Ok(Self::compute_proof(&data, index))
        }
    }

    /// Recursively compute a merkle tree.
    fn compute_proof(data: &[[u8; BlockId::LENGTH]], index: usize) -> MerkleProof {
        let n = data.len();
        debug_assert!(index < n);
        match n {
            0 | 1 => unreachable!(),
            // The terminating point, where we only have two values that become
            // left and right leaves. The chosen index is a `Value` while
            // the other is a `Node`.
            2 => {
                let (l, r) = (data[0], data[1]);
                if index == 0 {
                    MerkleProof {
                        left: Hashable::Value(Self::hash_leaf(l)),
                        right: Hashable::Node(Self::hash_leaf(r)),
                    }
                } else {
                    MerkleProof {
                        left: Hashable::Node(Self::hash_leaf(l)),
                        right: Hashable::Value(Self::hash_leaf(r)),
                    }
                }
            }
            _ => {
                // Split the blocks into two halves, ensuring that the tree is approximately balanced
                let k = largest_power_of_two(n);
                // If the chosen index is in the left half of the tree,
                // we build out that structure by calling this fn recursively.
                // Otherwise, we simply hash the subtree and store it as a `Node`.
                if index < k {
                    MerkleProof {
                        left: if data[..k].len() == 1 {
                            Hashable::Value(Self::hash_leaf(data[0]))
                        } else {
                            Hashable::MerkleProof(Box::new(Self::compute_proof(&data[..k], index)))
                        },
                        right: Hashable::Node(Self::hash(&data[k..])),
                    }
                } else {
                    MerkleProof {
                        left: Hashable::Node(Self::hash(&data[..k])),
                        right: if data[k..].len() == 1 {
                            Hashable::Value(Self::hash_leaf(data[k]))
                        } else {
                            Hashable::MerkleProof(Box::new(Self::compute_proof(&data[k..], index - k)))
                        },
                    }
                }
            }
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chronicle::types::stardust::block::BlockId;

    use super::*;

    #[test]
    fn test_compute_proof() {
        let block_ids = [
            "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c649",
            "0x81855ad8681d0d86d1e91e00167939cb6694d2c422acd208a0072939487f6999",
            "0xeb9d18a44784045d87f3c67cf22746e995af5a25367951baa2ff6cd471c483f1",
            "0x5fb90badb37c5821b6d95526a41a9504680b4e7c8b763a1b1d49d4955c848621",
            "0x6325253fec738dd7a9e28bf921119c160f0702448615bbda08313f6a8eb668d2",
            "0x0bf5059875921e668a5bdf2c7fc4844592d2572bcd0668d2d6c52f5054e2d083",
            "0x6bf84c7174cb7476364cc3dbd968b0f7172ed85794bb358b0c3b525da1786f9f",
        ]
        .iter()
        .map(|hash| BlockId::from_str(hash).unwrap())
        .collect::<Vec<_>>();

        let inclusion_merkle_root = MerkleHasher::hash_block_ids(&block_ids);

        for index in 0..block_ids.len() {
            let proof = MerkleHasher::create_proof_from_index(&block_ids, index).unwrap();
            let hash = proof.hash();

            assert_eq!(
                proof,
                MerkleProofDto::from(proof.clone()).try_into().unwrap(),
                "proof dto roundtrip"
            );
            assert_eq!(inclusion_merkle_root, hash, "proof hash doesn't equal the merkle root");
            assert!(
                proof.contains_block_id(&block_ids[index]),
                "proof does not contain that block id"
            );
        }
    }
}
