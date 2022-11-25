// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{blake2b::Blake2b256, Output};
use serde::{Deserialize, Serialize};

use super::{error::CreateProofError, merkle_hasher::MerkleHasher};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerklePath {
    left: Hashable,
    right: Hashable,
}

impl MerklePath {
    pub fn hash(&self, hasher: &MerkleHasher<Blake2b256>) -> Output<Blake2b256> {
        hasher.hash_node(self.left.hash(hasher), self.right.hash(hasher))
    }

    pub fn contains_block_id(&self, block_id: &BlockId) -> bool {
        self.left.contains_block_id(block_id) || self.right.contains_block_id(block_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Hashable {
    Path(Box<MerklePath>),
    Node(Output<Blake2b256>),
    Value([u8; BlockId::LENGTH]),
}

impl Hashable {
    fn hash(&self, hasher: &MerkleHasher<Blake2b256>) -> Output<Blake2b256> {
        match self {
            Hashable::Node(hash) => *hash,
            Hashable::Path(path) => path.hash(hasher),
            Hashable::Value(block_id) => hasher.hash_leaf(block_id),
        }
    }

    fn contains_block_id(&self, block_id: &BlockId) -> bool {
        match self {
            Hashable::Node(_) => false,
            Hashable::Path(path) => (*path).contains_block_id(block_id),
            Hashable::Value(v) => v == &block_id.0,
        }
    }
}

impl MerkleHasher<Blake2b256> {
    pub fn create_proof(&self, block_ids: &[BlockId], block_id: &BlockId) -> Result<MerklePath, CreateProofError> {
        if block_ids.len() < 2 {
            Err(CreateProofError::InsufficientBlockIds(block_ids.len()))
        } else {
            let index =
                find_index(block_ids, block_id).ok_or_else(|| CreateProofError::BlockNotIncluded(block_id.to_hex()))?;
            Ok(self.create_proof_from_index(block_ids, index))
        }
    }

    // NOTE:
    // * `block_ids` is the list of past-cone block ids in "White Flag" order;
    // * `block_ids.len() >= 2` must be true, or this function panics;
    // * `index < block_ids.len()` must be true, or this function panics;
    fn create_proof_from_index(&self, block_ids: &[BlockId], index: usize) -> MerklePath {
        let n = block_ids.len();
        debug_assert!(index < n);

        let data = block_ids.iter().map(|block_id| block_id.0).collect::<Vec<_>>();
        let proof = self.compute_proof(&data, index);
        if let Hashable::Path(proof) = proof {
            *proof
        } else {
            // The root of this recursive structure will always be `Hashable::MerkleProof`.
            unreachable!();
        }
    }

    /// TODO
    fn compute_proof(&self, data: &[[u8; BlockId::LENGTH]], index: usize) -> Hashable {
        let n = data.len();
        debug_assert!(index < n);
        match n {
            0 => unreachable!("empty data"),
            1 => Hashable::Value(data[0]),
            2 => {
                let (l, r) = (data[0], data[1]);
                let proof = if index == 0 {
                    MerklePath {
                        left: Hashable::Value(l),
                        right: Hashable::Node(self.hash_leaf(r)),
                    }
                } else {
                    MerklePath {
                        left: Hashable::Node(self.hash_leaf(l)),
                        right: Hashable::Value(r),
                    }
                };
                Hashable::Path(Box::new(proof))
            }
            _ => {
                let k = super::merkle_hasher::largest_power_of_two(n);
                let proof = if index < k {
                    MerklePath {
                        left: self.compute_proof(&data[..k], index),
                        right: Hashable::Node(self.hash(&data[k..])),
                    }
                } else {
                    MerklePath {
                        left: Hashable::Node(self.hash(&data[..k])),
                        right: self.compute_proof(&data[k..], index - k),
                    }
                };
                Hashable::Path(Box::new(proof))
            }
        }
    }
}

fn find_index(block_ids: &[BlockId], block_id: &BlockId) -> Option<usize> {
    block_ids.iter().position(|id| id == block_id)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerklePathDto {
    #[serde(rename = "l")]
    left: HashableDto,
    #[serde(rename = "r")]
    right: HashableDto,
}

impl From<MerklePath> for MerklePathDto {
    fn from(value: MerklePath) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}

impl TryFrom<MerklePathDto> for MerklePath {
    type Error = prefix_hex::Error;

    fn try_from(proof: MerklePathDto) -> Result<Self, Self::Error> {
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
    Path(Box<MerklePathDto>),
    Value {
        #[serde(rename = "value")]
        block_id_hex: String,
    },
}

impl From<Hashable> for HashableDto {
    fn from(value: Hashable) -> Self {
        match value {
            Hashable::Node(hash) => Self::Node {
                hash: prefix_hex::encode(hash.as_slice()),
            },
            Hashable::Path(path) => Self::Path(Box::new((*path).into())),
            Hashable::Value(block_id) => Self::Value {
                block_id_hex: prefix_hex::encode(block_id.as_slice()),
            },
        }
    }
}

impl TryFrom<HashableDto> for Hashable {
    type Error = prefix_hex::Error;

    fn try_from(hashed: HashableDto) -> Result<Self, Self::Error> {
        use iota_types::block::payload::milestone::MerkleRoot;
        Ok(match hashed {
            HashableDto::Node { hash } => Hashable::Node(prefix_hex::decode::<[u8; MerkleRoot::LENGTH]>(&hash)?.into()),
            HashableDto::Path(path) => Hashable::Path(Box::new(MerklePath::try_from(*path)?)),
            HashableDto::Value { block_id_hex } => {
                Hashable::Value(prefix_hex::decode::<[u8; BlockId::LENGTH]>(&block_id_hex)?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crypto::hashes::blake2b::Blake2b256;

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

        let hasher = MerkleHasher::<Blake2b256>::new();
        let inclusion_merkle_root = hasher.hash_block_ids(&block_ids);

        for index in 0..block_ids.len() {
            let proof = hasher.create_proof_from_index(&block_ids, index);
            let hash = proof.hash(&hasher);

            assert_eq!(
                proof,
                MerklePathDto::from(proof.clone()).try_into().unwrap(),
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
