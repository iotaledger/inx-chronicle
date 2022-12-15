// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use serde::{Deserialize, Serialize};

use super::{
    error::CreateProofError,
    merkle_hasher::{MerkleHash, MerkleHasher},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleAuditPath {
    left: Hashable,
    right: Option<Hashable>,
}

impl MerkleAuditPath {
    pub fn hash(&self) -> MerkleHash {
        // Handle edge case where the Merkle Tree consists solely of the "value".
        if self.left.is_value() && self.right.is_none() {
            self.left.hash()
        } else {
            // We make sure that unwrapping is safe.
            MerkleHasher::hash_node(self.left.hash(), self.right.as_ref().unwrap().hash())
        }
    }

    pub fn contains_block_id(&self, block_id: &BlockId) -> bool {
        self.left.contains_block_id(block_id) || self.right.as_ref().unwrap().contains_block_id(block_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Hashable {
    Path(Box<MerkleAuditPath>),
    Node(MerkleHash),
    Value([u8; BlockId::LENGTH]),
}

impl Hashable {
    fn hash(&self) -> MerkleHash {
        match self {
            Hashable::Node(hash) => *hash,
            Hashable::Path(path) => path.hash(),
            Hashable::Value(block_id) => MerkleHasher::hash_leaf(block_id),
        }
    }

    fn contains_block_id(&self, block_id: &BlockId) -> bool {
        match self {
            Hashable::Node(_) => false,
            Hashable::Path(path) => (*path).contains_block_id(block_id),
            Hashable::Value(v) => v == &block_id.0,
        }
    }

    fn is_value(&self) -> bool {
        matches!(self, Hashable::Value(_))
    }
}

pub struct MerkleProof;

impl MerkleProof {
    /// Creates the Merkle Tree audit path for a `block_id` contained in a list of `block_ids` sorted by their
    /// White-Flag index.
    ///
    /// Returns an error if the given `block_id` is not actually part of the also given `block_ids` list.
    pub fn create_audit_path(block_ids: &[BlockId], block_id: &BlockId) -> Result<MerkleAuditPath, CreateProofError> {
        // Get index of the block id in the list of block ids.
        let index = block_ids
            .iter()
            .position(|id| id == block_id)
            .ok_or_else(|| CreateProofError::BlockNotIncluded(block_id.to_hex()))?;

        Ok(Self::create_audit_path_from_index(block_ids, index))
    }

    fn create_audit_path_from_index(block_ids: &[BlockId], index: usize) -> MerkleAuditPath {
        let block_ids = block_ids.iter().map(|block_id| block_id.0).collect::<Vec<_>>();
        Self::compute_audit_path(&block_ids, index)
    }

    // Recursive function that deterministically computes the Merkle Tree audit path for a certain `BlockId`
    // in a list of ordered and unique `BlockId`s. It is the responsibility of the caller to make sure those
    // invariants are upheld.
    //
    // For further details on the usage of Merkle trees and Proof of Inclusion in IOTA, have a look at:
    // [TIP-0004](https://github.com/iotaledger/tips/blob/main/tips/TIP-0004/tip-0004.md).
    fn compute_audit_path(block_ids: &[[u8; BlockId::LENGTH]], index: usize) -> MerkleAuditPath {
        let n = block_ids.len();
        debug_assert!(n > 0 && index < n, "n={n}, index={index}");

        // Handle the special case where the Merkle Tree consists solely of the "value".
        if n == 1 {
            return MerkleAuditPath {
                left: Hashable::Value(block_ids[0]),
                right: None,
            };
        }

        // Select a `pivot` element to split `data` into two slices `left` and `right`.
        let pivot = super::merkle_hasher::largest_power_of_two(n);
        let (left, right) = block_ids.split_at(pivot);

        // Produces the Merkle hash of a sub tree not containing the `value`.
        let subtree_hash = |block_ids| Hashable::Node(MerkleHasher::hash(block_ids));

        // Produces the Merkle audit path for the given `value`.
        let subtree_with_value = |block_ids: &[[u8; BlockId::LENGTH]], index| {
            if block_ids.len() == 1 {
                Hashable::Value(block_ids[0])
            } else {
                Hashable::Path(Box::new(Self::compute_audit_path(block_ids, index)))
            }
        };

        if index < pivot {
            // `value` is contained in the left subtree, and the `right` subtree can be hashed together.
            MerkleAuditPath {
                left: subtree_with_value(left, index),
                right: Some(subtree_hash(right)),
            }
        } else {
            // `value` is contained in the right subtree, and the `left` subtree can be hashed together.
            MerkleAuditPath {
                left: subtree_hash(left),
                right: Some(subtree_with_value(right, index - pivot)),
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleAuditPathDto {
    #[serde(rename = "l")]
    left: HashableDto,
    #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
    right: Option<HashableDto>,
}

impl From<MerkleAuditPath> for MerkleAuditPathDto {
    fn from(value: MerkleAuditPath) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.map(|v| v.into()),
        }
    }
}

impl TryFrom<MerkleAuditPathDto> for MerkleAuditPath {
    type Error = prefix_hex::Error;

    fn try_from(proof: MerkleAuditPathDto) -> Result<Self, Self::Error> {
        Ok(Self {
            left: Hashable::try_from(proof.left)?,
            right: proof.right.map(Hashable::try_from).transpose()?,
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
    Path(Box<MerkleAuditPathDto>),
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
    fn test_create_audit_path() {
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

        let expected_merkle_root = MerkleHasher::hash_block_ids(&block_ids);

        for (index, block_id) in block_ids.iter().enumerate() {
            let audit_path = MerkleProof::create_audit_path(&block_ids, block_id).unwrap();
            let audit_path_merkle_root = audit_path.hash();

            assert_eq!(
                audit_path,
                MerkleAuditPathDto::from(audit_path.clone()).try_into().unwrap(),
                "audit path dto roundtrip"
            );
            assert_eq!(
                expected_merkle_root, audit_path_merkle_root,
                "audit path hash doesn't equal the merkle root"
            );
            assert!(
                audit_path.contains_block_id(&block_ids[index]),
                "audit path does not contain that block id"
            );
        }
    }

    #[test]
    fn test_create_audit_path_for_single_block() {
        let block_id = BlockId::from_str("0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c649").unwrap();
        let block_ids = vec![block_id];
        let expected_merkle_root = MerkleHasher::hash_block_ids(&block_ids);
        let audit_path = MerkleProof::create_audit_path(&block_ids, &block_id).unwrap();
        let audit_path_merkle_root = audit_path.hash();

        assert_eq!(
            audit_path,
            MerkleAuditPathDto::from(audit_path.clone()).try_into().unwrap(),
            "audit path dto roundtrip"
        );
        assert_eq!(
            expected_merkle_root, audit_path_merkle_root,
            "audit path hash doesn't equal the merkle root"
        );
        assert!(
            audit_path.contains_block_id(&block_ids[0]),
            "audit path does not contain that block id"
        );
    }
}
