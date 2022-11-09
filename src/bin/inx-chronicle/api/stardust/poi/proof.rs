// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{blake2b::Blake2b256, Output};
use serde::{Deserialize, Serialize};

use super::{error::PoIError, hasher::MerkleTreeHasher};

type Hash = Vec<u8>;

#[derive(Debug)]
pub struct Proof {
    left: Node,
    right: Node,
}

#[derive(Debug)]
pub enum Node {
    Hash(Hash),
    Value(Hash),
    Path(Box<Proof>),
}

impl From<Proof> for ProofDto {
    fn from(value: Proof) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}

impl From<Node> for NodeDto {
    fn from(value: Node) -> Self {
        match value {
            Node::Hash(hash) => Self::Hash { hash: prefix_hex::encode(hash) },
            Node::Value(val) => Self::Value { val: prefix_hex::encode(val) },
            Node::Path(path) => Self::Path(Box::new((*path).into())),
        }
    }
}

impl TryFrom<ProofDto> for Proof {
    type Error = prefix_hex::Error;

    fn try_from(proof: ProofDto) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<NodeDto> for Node {
    type Error = prefix_hex::Error;

    fn try_from(node: NodeDto) -> Result<Self, Self::Error> {
        todo!()
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
        val: String
    },
    Path(Box<ProofDto>),
}

impl Proof {
    pub(crate) fn contains_block_id(&self, block_id: &BlockId) -> Result<bool, ()> {
        Ok(true)
    }

    pub(crate) fn hash(&self, hasher: &mut MerkleTreeHasher<Blake2b256>) -> &[u8] {
        todo!()
    }
}

impl MerkleTreeHasher<Blake2b256> {
    pub fn create_proof(&self, block_ids: Vec<BlockId>, block_id: &BlockId) -> Result<Proof, PoIError> {
        let index = find_index(&block_ids, block_id).ok_or(PoIError::InvalidRequest("invalid BlockId"))?;

        self.create_proof_from_index(block_ids, index)
    }

    // NOTE: `block_ids` is the list of past-cone block ids in "White Flag" order.
    fn create_proof_from_index(&self, block_ids: Vec<BlockId>, index: usize) -> Result<Proof, PoIError> {
        if block_ids.len() < 2 {
            return Err(PoIError::InvalidPrecondition(
                "block id list must have at least 2 items",
            ));
        }

        if index >= block_ids.len() {
            return Err(PoIError::InvalidRequest("index out of bounds"));
        }

        let data = block_ids
            .into_iter()
            .map(|block_id| block_id.0.to_vec())
            .collect::<Vec<_>>();
        let node = self.compute_proof(&data, index);

        if let Node::Path(proof) = node {
            Ok(*proof)
        } else {
            unreachable!("root node must be a path variant");
        }
    }

    // NOTE: The root "node" must be a `Node::Proof`.
    fn compute_proof(&self, data: &[Hash], index: usize) -> Node {
        let n = data.len();
        match n {
            0 => unreachable!(),
            1 => Node::Value(data[0].clone()),
            2 => {
                if index == 0 {
                    Node::Path(Box::new(Proof {
                        left: Node::Value(data[0].clone()),
                        right: Node::Hash(self.hash_leaf(&data[1]).to_vec()),
                    }))
                } else {
                    Node::Path(Box::new(Proof {
                        left: Node::Hash(self.hash_leaf(&data[0]).to_vec()),
                        right: Node::Value(data[1].clone()),
                    }))
                }
            }
            _ => {
                let k = super::hasher::largest_power_of_two_lte_number(n as u32 - 1);
                if index < k {
                    Node::Path(Box::new(Proof {
                        left: self.compute_proof(&data[..k], index),
                        right: Node::Hash(self.hash_node(&data[k..]).to_vec()),
                    }))
                } else {
                    Node::Path(Box::new(Proof {
                        left: Node::Hash(self.hash_node(&data[..k]).to_vec()),
                        right: self.compute_proof(&data[k..], index - k),
                    }))
                }
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
        .map(|hash| BlockId::from_str(hash).unwrap().0.to_vec())
        .collect::<Vec<_>>();

        let hasher = MerkleTreeHasher::<Blake2b256>::new();
        let index = 0;
        let node = hasher.compute_proof(&block_ids, index);
        if let Node::Path(proof) = node {
            let proof_dto = ProofDto::from(*proof);
            println!("{}", serde_json::to_string_pretty(&proof_dto).unwrap());
        } else {
            panic!("root should be a path")
        }
    }
}
