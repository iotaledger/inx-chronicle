// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::blake2b::Blake2b256;
use serde::{Deserialize, Serialize};

use super::{error::PoIError, hasher::MerkleHasher};

pub type Hash = Box<[u8]>;
pub type Hasher = MerkleHasher<Blake2b256>;

#[derive(Debug)]
pub struct Proof {
    left: Hashed,
    right: Hashed,
}

impl Proof {
    pub fn contains_block_id(&self, block_id: &BlockId) -> Result<bool, ()> {
        Ok(true)
    }

    pub fn hash(&self, hasher: &Hasher) -> Hash {
        let l = self.left.hash();
        let r = self.right.hash();
        hasher.hash_node(l, r).to_vec().into_boxed_slice()
    }
}

#[derive(Debug)]
pub enum Hashed {
    Node(Hash),
    Proof(Box<Proof>, Hash),
    Value(Hash),
}

impl Hashed {
    fn hash(&self) -> Hash {
        match self {
            Hashed::Node(h) | Hashed::Value(h) | Hashed::Proof(_, h) => h.clone(),
        }
    }
}

impl MerkleHasher<Blake2b256> {
    pub fn create_proof(&self, block_ids: Vec<BlockId>, block_id: &BlockId) -> Result<Proof, PoIError> {
        let index = find_index(&block_ids, block_id).ok_or(PoIError::InvalidRequest("invalid BlockId"))?;
        self.create_proof_from_index(block_ids, index)
    }

    // NOTE: `block_ids` is the list of past-cone block ids in "White Flag" order.
    fn create_proof_from_index(&self, block_ids: Vec<BlockId>, index: usize) -> Result<Proof, PoIError> {
        let n = block_ids.len();
        if n < 2 {
            Err(PoIError::InvalidPrecondition(
                "block id list must have at least 2 items",
            ))
        } else if index >= n {
            Err(PoIError::InvalidRequest("index out of bounds"))
        } else {
            let data = block_ids.into_iter().map(|block_id| block_id.0).collect::<Vec<_>>();
            let proof = self.compute_proof(&data, index);
            if let Hashed::Proof(proof, _) = proof {
                Ok(*proof)
            } else {
                unreachable!("root wasn't a proof");
            }
        }
    }

    fn compute_proof(&self, data: &[[u8; 32]], index: usize) -> Hashed {
        let n = data.len();
        debug_assert!(index < n);
        match n {
            0 => unreachable!("empty data given"),
            1 => Hashed::Value(self.hash_leaf(data[0]).to_vec().into_boxed_slice()),
            2 => {
                let (l, r) = (data[0], data[1]);
                let proof = if index == 0 {
                    Proof {
                        left: Hashed::Value(self.hash_leaf(l).to_vec().into_boxed_slice()),
                        right: Hashed::Node(self.hash_leaf(r).to_vec().into_boxed_slice()),
                    }
                } else {
                    Proof {
                        left: Hashed::Node(self.hash_leaf(l).to_vec().into_boxed_slice()),
                        right: Hashed::Value(self.hash_leaf(r).to_vec().into_boxed_slice()),
                    }
                };
                let hash = proof.hash(self);
                Hashed::Proof(Box::new(proof), hash)
            }
            _ => {
                let k = super::hasher::largest_power_of_two(n);
                let proof = if index < k {
                    Proof {
                        left: self.compute_proof(&data[..k], index),
                        right: Hashed::Node(self.hash(&data[k..]).to_vec().into_boxed_slice()),
                    }
                } else {
                    Proof {
                        left: Hashed::Node(self.hash(&data[..k]).to_vec().into_boxed_slice()),
                        right: self.compute_proof(&data[k..], index - k),
                    }
                };
                let hash = proof.hash(self);
                Hashed::Proof(Box::new(proof), hash)
            }
        }
    }

    pub(crate) fn validate_proof(&self, proof: Proof) -> Result<bool, PoIError> {
        Ok(false)
    }
}

fn find_index(block_ids: &[BlockId], block_id: &BlockId) -> Option<usize> {
    block_ids.iter().position(|id| id == block_id)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofDto {
    #[serde(rename = "l")]
    left: HashedDto,
    #[serde(rename = "r")]
    right: HashedDto,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HashedDto {
    Node {
        #[serde(rename = "h")]
        hash: String,
    },
    Proof(Box<ProofDto>),
    Value {
        #[serde(rename = "value")]
        value_hash: String,
    },
}

impl From<Proof> for ProofDto {
    fn from(value: Proof) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}

impl From<Hashed> for HashedDto {
    fn from(value: Hashed) -> Self {
        match value {
            Hashed::Node(h) => Self::Node {
                hash: prefix_hex::encode(h),
            },
            Hashed::Value(v) => Self::Value {
                value_hash: prefix_hex::encode(v),
            },
            Hashed::Proof(p, _) => Self::Proof(Box::new((*p).into())),
        }
    }
}

impl TryFrom<ProofDto> for Proof {
    type Error = prefix_hex::Error;

    fn try_from(proof: ProofDto) -> Result<Self, Self::Error> {
        Ok(Self {
            left: Hashed::try_from(proof.left)?,
            right: Hashed::try_from(proof.right)?,
        })
    }
}

impl TryFrom<HashedDto> for Hashed {
    type Error = prefix_hex::Error;

    fn try_from(hashed: HashedDto) -> Result<Self, Self::Error> {
        Ok(match hashed {
            HashedDto::Node { hash } => {
                let hash = prefix_hex::decode::<[u8; 32]>(&hash)?;
                Hashed::Node(Box::new(hash))
            }
            HashedDto::Value { value_hash } => {
                let hash = prefix_hex::decode::<[u8; 32]>(&value_hash)?;
                Hashed::Value(Box::new(hash))
            }
            HashedDto::Proof(proof) => {
                let proof = Proof::try_from(*proof)?;
                let hasher = MerkleHasher::<Blake2b256>::new();
                let hash = proof.hash(&hasher);
                Hashed::Proof(Box::new(proof), hash)
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
        .map(|hash| BlockId::from_str(hash).unwrap().0)
        .collect::<Vec<_>>();

        let hasher = MerkleHasher::<Blake2b256>::new();
        let inclusion_merkle_root = hasher.hash(&block_ids).to_vec().into_boxed_slice();
        // println!("inclusion_merkle_root = {}", prefix_hex::encode(&inclusion_merkle_root[..]));

        for index in 0..block_ids.len() {
            let node = hasher.compute_proof(&block_ids, index);
            if let Hashed::Proof(proof, hash) = node {
                // println!("proof={}", prefix_hex::encode((*proof).hash(&hasher)));

                // let proof_dto = ProofDto::from(*proof);
                // println!("{}", serde_json::to_string_pretty(&proof_dto).unwrap());

                // assert_eq!(inclusion_merkle_root, (*proof).hash(&hasher));
                assert_eq!(inclusion_merkle_root, hash);
            } else {
                panic!("root should be a path")
            }
        }
    }
}
