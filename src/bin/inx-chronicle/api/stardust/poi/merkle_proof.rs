// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{blake2b::Blake2b256, Output};
use serde::{Deserialize, Serialize};

use super::{error::CreateProofError, merkle_hasher::MerkleHasher};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleAuditPath {
    left: Hashable,
    right: Hashable,
}

impl MerkleAuditPath {
    pub fn hash(&self, hasher: &MerkleHasher<Blake2b256>) -> Output<Blake2b256> {
        hasher.hash_node(self.left.hash(hasher), self.right.hash(hasher))
    }

    pub fn contains_block_id(&self, block_id: &BlockId) -> bool {
        self.left.contains_block_id(block_id) || self.right.contains_block_id(block_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Hashable {
    Path(Box<MerkleAuditPath>),
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

pub struct MerkleProof {
    pub hasher: MerkleHasher<Blake2b256>,
}

impl MerkleProof {
    pub fn new() -> Self {
        Self {
            hasher: MerkleHasher::new(),
        }
    }

    pub fn create_audit_path(
        &self,
        block_ids: &[BlockId],
        block_id: &BlockId,
    ) -> Result<MerkleAuditPath, CreateProofError> {
        if block_ids.len() < 2 {
            Err(CreateProofError::InsufficientBlockIds(block_ids.len()))
        } else {
            let index =
                find_index(block_ids, block_id).ok_or_else(|| CreateProofError::BlockNotIncluded(block_id.to_hex()))?;
            Ok(self.create_audit_path_from_index(block_ids, index))
        }
    }

    // NOTE:
    // * `block_ids` is the list of past-cone block ids in "White Flag" order;
    // * `block_ids.len() >= 2` must be true, or this function panics;
    // * `index < block_ids.len()` must be true, or this function panics;
    pub fn create_audit_path_from_index(&self, block_ids: &[BlockId], index: usize) -> MerkleAuditPath {
        let n = block_ids.len();
        debug_assert!(index < n);

        let data = block_ids.iter().map(|block_id| block_id.0).collect::<Vec<_>>();
        self.compute_audit_path(&data, index)
    }

    /// Recursively computes the "Merkle Audit Path" for a certain `BlockId` that is given by its index in a list of
    /// ordered and unique `BlockId`s.
    ///
    /// Given `n` block ids, the algorithm deterministically selects a `mid` (largest power of two
    /// less than `n`), at which it splits the data into a `left` and a `right` side ranging from `[0...mid)` and
    /// `[mid..n)` repectively. Depending on which range `index` belongs to (i.e. the location of the `BlockId` in the
    /// binary subtree), the function recursively calls itself on the associated slice and creates the Merkle tree
    /// hash for the opposite side/subtree. It terminates if either of two things occur: (a) the given slice consists
    /// only of 2 elements; in that case - depending on `index` - one is the `value` leaf and the other a hash of a
    /// leaf; (b) after splitting `data` into two slices/subtrees (as described above), one side/subtree just
    /// consists of the `value` leaf and the other of a hash of a Merkle sub tree not containing the `value` leaf.
    ///
    /// For further details on Merkle trees, Merkle audit paths and Proof of Inclusion have a look at:
    /// [TIP-0004](https://github.com/iotaledger/tips/blob/main/tips/TIP-0004/tip-0004.md) for more details.
    fn compute_audit_path(&self, data: &[[u8; BlockId::LENGTH]], index: usize) -> MerkleAuditPath {
        let n = data.len();
        debug_assert!(n > 1 && index < n);
        match n {
            0 | 1 => unreachable!("invalid input data"),
            2 => {
                let (left, right) = (data[0], data[1]);
                if index == 0 {
                    MerkleAuditPath {
                        left: Hashable::Value(left),
                        right: Hashable::Node(self.hasher.hash_leaf(right)),
                    }
                } else {
                    MerkleAuditPath {
                        left: Hashable::Node(self.hasher.hash_leaf(left)),
                        right: Hashable::Value(right),
                    }
                }
            }
            _ => {
                let mid = super::merkle_hasher::largest_power_of_two(n);
                let (left, right) = data.split_at(mid);
                if index < mid {
                    MerkleAuditPath {
                        left: if left.len() == 1 {
                            Hashable::Value(left[0])
                        } else {
                            Hashable::Path(Box::new(self.compute_audit_path(left, index)))
                        },
                        right: Hashable::Node(self.hasher.hash(right)),
                    }
                } else {
                    MerkleAuditPath {
                        left: Hashable::Node(self.hasher.hash(left)),
                        right: if right.len() == 1 {
                            Hashable::Value(right[0])
                        } else {
                            Hashable::Path(Box::new(self.compute_audit_path(right, index - mid)))
                        },
                    }
                }
            }
        }
    }

    pub fn get_merkle_root(&self, merkle_audit_path: &MerkleAuditPath) -> Output<Blake2b256> {
        merkle_audit_path.hash(&self.hasher)
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

impl From<MerkleAuditPath> for MerklePathDto {
    fn from(value: MerkleAuditPath) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}

impl TryFrom<MerklePathDto> for MerkleAuditPath {
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
            HashableDto::Path(path) => Hashable::Path(Box::new(MerkleAuditPath::try_from(*path)?)),
            HashableDto::Value { block_id_hex } => {
                Hashable::Value(prefix_hex::decode::<[u8; BlockId::LENGTH]>(&block_id_hex)?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

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

        let proof = MerkleProof::new();
        let expected_merkle_root = proof.hasher.hash_block_ids(&block_ids);

        for index in 0..block_ids.len() {
            let merkle_audit_path = proof.create_audit_path_from_index(&block_ids, index);
            let calculated_merkle_root = proof.get_merkle_root(&merkle_audit_path);

            assert_eq!(
                merkle_audit_path,
                MerklePathDto::from(merkle_audit_path.clone()).try_into().unwrap(),
                "proof dto roundtrip"
            );
            assert_eq!(
                expected_merkle_root, calculated_merkle_root,
                "proof hash doesn't equal the merkle root"
            );
            assert!(
                merkle_audit_path.contains_block_id(&block_ids[index]),
                "proof does not contain that block id"
            );
        }
    }
}
