// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{blake2b::Blake2b256, Output};
use serde::{Deserialize, Serialize};

use super::{error::PoIError, merkle_hasher::MerkleHasher};

#[derive(Debug)]
pub struct MerkleProof {
    left: Hashable,
    right: Hashable,
}

impl MerkleProof {
    pub fn hash(&self, hasher: &MerkleHasher<Blake2b256>) -> Output<Blake2b256> {
        let l = self.left.hash();
        let r = self.right.hash();
        hasher.hash_node(l, r)
    }

    pub fn contains_block_id(&self, block_id: &BlockId, hasher: &MerkleHasher<Blake2b256>) -> bool {
        let value = hasher.hash_leaf(block_id.0);
        self.contains_value(&value)
    }

    pub fn contains_value(&self, value: &impl AsRef<[u8]>) -> bool {
        self.left.contains_value(value) || self.right.contains_value(value)
    }
}

#[derive(Debug)]
pub enum Hashable {
    MerkleProof(Box<MerkleProof>, Output<Blake2b256>),
    Node(Output<Blake2b256>),
    Value(Output<Blake2b256>),
}

impl Hashable {
    fn hash(&self) -> Output<Blake2b256> {
        match self {
            Hashable::MerkleProof(_, hash) | Hashable::Node(hash) | Hashable::Value(hash) => *hash,
        }
    }

    fn contains_value(&self, value: &impl AsRef<[u8]>) -> bool {
        match self {
            Hashable::MerkleProof(p, _) => (*p).contains_value(value),
            Hashable::Node(_) => false,
            Hashable::Value(v) => &**v == value.as_ref(),
        }
    }
}

impl MerkleHasher<Blake2b256> {
    pub fn create_proof(&self, block_ids: &[BlockId], block_id: &BlockId) -> Result<MerkleProof, PoIError> {
        let index = find_index(block_ids, block_id).ok_or(PoIError::InvalidRequest("invalid BlockId"))?;
        self.create_proof_from_index(block_ids, index)
    }

    // NOTE: `block_ids` is the list of past-cone block ids in "White Flag" order.
    fn create_proof_from_index(&self, block_ids: &[BlockId], index: usize) -> Result<MerkleProof, PoIError> {
        let n = block_ids.len();
        if n < 2 {
            Err(PoIError::InvalidInput("cannot create proof for less than 2 block ids"))
        } else if index >= n {
            Err(PoIError::InvalidInput("given index is out of bounds"))
        } else {
            let data = block_ids.iter().map(|block_id| block_id.0).collect::<Vec<_>>();
            let proof = self.compute_proof(&data, index);
            if let Hashable::MerkleProof(proof, _) = proof {
                Ok(*proof)
            } else {
                // The root of this recursive structure will always be `Hashable::MerkleProof`.
                unreachable!();
            }
        }
    }

    fn compute_proof(&self, data: &[[u8; BlockId::LENGTH]], index: usize) -> Hashable {
        let n = data.len();
        debug_assert!(index < n);
        match n {
            0 => unreachable!("empty data"),
            1 => Hashable::Value(self.hash_leaf(data[0])),
            2 => {
                let (l, r) = (data[0], data[1]);
                let proof = if index == 0 {
                    MerkleProof {
                        left: Hashable::Value(self.hash_leaf(l)),
                        right: Hashable::Node(self.hash_leaf(r)),
                    }
                } else {
                    MerkleProof {
                        left: Hashable::Node(self.hash_leaf(l)),
                        right: Hashable::Value(self.hash_leaf(r)),
                    }
                };
                let hash = proof.hash(self);
                Hashable::MerkleProof(Box::new(proof), hash)
            }
            _ => {
                let k = super::merkle_hasher::largest_power_of_two(n);
                let proof = if index < k {
                    MerkleProof {
                        left: self.compute_proof(&data[..k], index),
                        right: Hashable::Node(self.hash(&data[k..])),
                    }
                } else {
                    MerkleProof {
                        left: Hashable::Node(self.hash(&data[..k])),
                        right: self.compute_proof(&data[k..], index - k),
                    }
                };
                let hash = proof.hash(self);
                Hashable::MerkleProof(Box::new(proof), hash)
            }
        }
    }
}

fn find_index(block_ids: &[BlockId], block_id: &BlockId) -> Option<usize> {
    block_ids.iter().position(|id| id == block_id)
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
            Hashable::MerkleProof(p, _) => Self::Proof(Box::new((*p).into())),
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
                let hasher = MerkleHasher::<Blake2b256>::new();
                let hash = proof.hash(&hasher);
                Hashable::MerkleProof(Box::new(proof), hash)
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
            let proof = hasher.create_proof_from_index(&block_ids, index).unwrap();
            let hash = proof.hash(&hasher);

            // let proof_dto = ProofDto::from(*proof);
            // println!("{}", serde_json::to_string_pretty(&proof_dto).unwrap());

            assert_eq!(inclusion_merkle_root, hash, "proof hash doesn't equal the merkle root");
            assert!(
                proof.contains_block_id(&block_ids[index], &hasher),
                "proof does not contain that block id"
            );
        }
    }
}
