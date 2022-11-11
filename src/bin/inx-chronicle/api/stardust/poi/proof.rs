// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{blake2b::Blake2b256, Digest, Output};
use serde::{Deserialize, Serialize};

use super::{error::PoIError, hasher::MerkleHasher};

pub type Bytes = Box<[u8]>;
pub type Hasher = MerkleHasher<Blake2b256>;

#[derive(Debug)]
pub struct Proof {
    left: Hashable,
    right: Hashable,
}

impl Proof {
    pub fn contains_block_id(&self, block_id: &BlockId) -> Result<bool, ()> {
        Ok(true)
    }

    pub fn hash(&self, hasher: &Hasher) -> Bytes {
        let l = self.left.hash(hasher);
        let r = self.right.hash(hasher);
        hasher.hash_node(l, r).to_vec().into_boxed_slice()
    }
}

#[derive(Debug)]
pub enum Hashable {
    Tree(Bytes),
    Value(Bytes),
    Proof(Box<Proof>),
}

impl Hashable {
    fn hash(&self, hasher: &Hasher) -> Bytes {
        match self {
            Hashable::Tree(h) | Hashable::Value(h) => h.clone(),
            Hashable::Proof(p) => (**p).hash(hasher),
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
            if let Hashable::Proof(proof) = proof {
                Ok(*proof)
            } else {
                unreachable!("root wasn't a proof");
            }
        }
    }

    fn compute_proof(&self, mut data: &[[u8; 32]], index: usize) -> Hashable {
        let n = data.len();
        debug_assert!(index < n);
        match n {
            0 => unreachable!("empty data given"),
            1 => Hashable::Value(Box::new(data[0])),
            2 => {
                let (l, r) = (data[0], data[1]);
                let proof = if index == 0 {
                    Proof {
                        left: Hashable::Value(Box::new(l)),
                        right: Hashable::Tree(self.hash_leaf(r).to_vec().into_boxed_slice()),
                    }
                } else {
                    Proof {
                        left: Hashable::Tree(self.hash_leaf(l).to_vec().into_boxed_slice()),
                        right: Hashable::Value(Box::new(r)),
                    }
                };
                Hashable::Proof(Box::new(proof))
            }
            _ => {
                let k = super::hasher::largest_power_of_two(n);
                let proof = if index < k {
                    Proof {
                        left: self.compute_proof(&data[..k], index),
                        right: Hashable::Tree(self.hash(&data[k..]).to_vec().into_boxed_slice()),
                    }
                } else {
                    Proof {
                        left: Hashable::Tree(self.hash(&data[..k]).to_vec().into_boxed_slice()),
                        right: self.compute_proof(&data[k..], index - k),
                    }
                };
                Hashable::Proof(Box::new(proof))
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
        // let index = 4;

        for index in 0..block_ids.len() {
            let node = hasher.compute_proof(&block_ids, index);
            if let Hashable::Proof(proof) = node {
                println!("{}", prefix_hex::encode((*proof).hash(&hasher)));

                // let proof_dto = ProofDto::from(*proof);
                // println!("{}", serde_json::to_string_pretty(&proof_dto).unwrap());
            } else {
                panic!("root should be a path")
            }
        }
    }
}

#[cfg(feature = "rand")]
mod test_rand {
    use chronicle::types::stardust::block::payload::{MilestoneId, MilestonePayload};

    #[tokio::test]
    async fn test_foo() {
        let milestone = MilestonePayload::rand(&iota_types::block::protocol::protocol_parameters());
        let milestone_id = MilestoneId::rand();

        println!("{0:?}", prefix_hex::encode(milestone.essence.inclusion_merkle_root));
        println!("{0}", milestone_id.to_hex());
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofDto {
    #[serde(rename = "l")]
    left: NodeDto,
    #[serde(rename = "r")]
    right: NodeDto,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodeDto {
    Hash {
        #[serde(rename = "h")]
        hash: String,
    },
    Value {
        #[serde(rename = "value")]
        val: String,
    },
    Path(Box<ProofDto>),
}

impl From<Proof> for ProofDto {
    fn from(value: Proof) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}

impl From<Hashable> for NodeDto {
    fn from(value: Hashable) -> Self {
        match value {
            Hashable::Tree(h) => Self::Hash {
                hash: prefix_hex::encode(h),
            },
            Hashable::Value(v) => Self::Value {
                val: prefix_hex::encode(v),
            },
            Hashable::Proof(path) => Self::Path(Box::new((*path).into())),
        }
    }
}

impl TryFrom<ProofDto> for Proof {
    type Error = prefix_hex::Error;

    fn try_from(proof: ProofDto) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<NodeDto> for Hashable {
    type Error = prefix_hex::Error;

    fn try_from(node: NodeDto) -> Result<Self, Self::Error> {
        todo!()
    }
}
