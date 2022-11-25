// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

use chronicle::types::stardust::block::BlockId;

use super::{error::CreateAuditPathError, largest_power_of_two, MerkleHash, MerkleHasher};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleAuditPath {
    pub left: Hashable,
    pub right: Hashable,
}

impl MerkleAuditPath {
    pub fn hash(&self) -> MerkleHash {
        MerkleHasher::hash_node(self.left.hash().as_ref(), self.right.hash().as_ref())
    }

    pub fn contains_block_id(&self, block_id: &BlockId) -> bool {
        self.left.contains_block_id(block_id) || self.right.contains_block_id(block_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hashable {
    Path(Box<MerkleAuditPath>),
    Node(MerkleHash),
    Value(BlockId),
}

impl Hashable {
    fn hash(&self) -> Cow<MerkleHash> {
        match self {
            Hashable::Path(path) => Cow::Owned(path.hash()),
            Hashable::Node(hash) => Cow::Borrowed(hash),
            Hashable::Value(block_id) => Cow::Owned(MerkleHasher::hash_leaf(block_id)),
        }
    }

    fn contains_block_id(&self, block_id: &BlockId) -> bool {
        match self {
            Hashable::Node(_) => false,
            Hashable::Path(path) => path.contains_block_id(block_id),
            Hashable::Value(v) => v == block_id,
        }
    }
}

impl MerkleHasher {
    /// Create a merkle audit path given a list of block IDs and a chosen block ID. The chosen leaf will become a
    /// value node, and the path will contain all hashes above it. The remaining branches will be terminated early.
    pub fn create_audit_path(
        block_ids: &[BlockId],
        chosen_block_id: &BlockId,
    ) -> Result<MerkleAuditPath, CreateAuditPathError> {
        let index = block_ids
            .iter()
            .position(|id| id == chosen_block_id)
            .ok_or_else(|| CreateAuditPathError::BlockNotIncluded(chosen_block_id.to_hex()))?;
        Self::create_audit_path_from_index(block_ids, index)
    }

    // NOTE:
    // * `block_ids` is the list of past-cone block ids in "White Flag" order;
    // * `block_ids.len() >= 2` must be true, or this function panics;
    // * `index < block_ids.len()` must be true, or this function panics;
    fn create_audit_path_from_index(
        block_ids: &[BlockId],
        index: usize,
    ) -> Result<MerkleAuditPath, CreateAuditPathError> {
        if block_ids.len() < 2 {
            Err(CreateAuditPathError::InsufficientBlockIds(block_ids.len()))
        } else {
            let n = block_ids.len();
            debug_assert!(index < n);
            Ok(Self::compute_audit_path(block_ids, index))
        }
    }

    /// Recursively computes the "Merkle Audit Path" for a certain `BlockId` that is given by its index in a list of
    /// ordered and unique `BlockId`s.
    ///
    /// For further details on Merkle trees, Merkle audit paths and Proof of Inclusion have a look at:
    /// [TIP-0004](https://github.com/iotaledger/tips/blob/main/tips/TIP-0004/tip-0004.md) for more details.
    fn compute_audit_path(data: &[BlockId], index: usize) -> MerkleAuditPath {
        let n = data.len();
        debug_assert!(n > 1 && index < n);
        match n {
            0 | 1 => unreachable!(),
            // The terminating point, where we only have two values that become
            // left and right leaves. The chosen index is a `Value` while
            // the other is a `Node`.
            2 => {
                let (l, r) = (data[0], data[1]);
                if index == 0 {
                    MerkleAuditPath {
                        left: Hashable::Value(l),
                        right: Hashable::Node(Self::hash_leaf(r)),
                    }
                } else {
                    MerkleAuditPath {
                        left: Hashable::Node(Self::hash_leaf(l)),
                        right: Hashable::Value(r),
                    }
                }
            }
            _ => {
                // Split the blocks into two halves, ensuring that the tree is approximately balanced
                let mid = largest_power_of_two(n);
                let (left, right) = data.split_at(mid);
                // If the chosen index is in the left half of the tree,
                // we build out that structure by calling this fn recursively.
                // Otherwise, we simply hash the subtree and store it as a `Node`.
                if index < mid {
                    MerkleAuditPath {
                        left: if left.len() == 1 {
                            Hashable::Value(left[0])
                        } else {
                            Hashable::Path(Box::new(Self::compute_audit_path(left, index)))
                        },
                        right: Hashable::Node(Self::hash(right)),
                    }
                } else {
                    MerkleAuditPath {
                        left: Hashable::Node(Self::hash(left)),
                        right: if right.len() == 1 {
                            Hashable::Value(right[0])
                        } else {
                            Hashable::Path(Box::new(Self::compute_audit_path(right, index - mid)))
                        },
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chronicle::types::stardust::block::BlockId;

    use super::*;
    use crate::api::stardust::poi::merkle::MerkleAuditPathDto;

    #[test]
    fn test_compute_audit_path() {
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

        for index in 0..block_ids.len() {
            let merkle_audit_path = MerkleHasher::create_audit_path_from_index(&block_ids, index).unwrap();
            let calculated_merkle_root = merkle_audit_path.hash();

            assert_eq!(
                merkle_audit_path,
                MerkleAuditPathDto::from(merkle_audit_path.clone()).try_into().unwrap(),
                "audit path dto roundtrip"
            );
            assert_eq!(
                expected_merkle_root, calculated_merkle_root,
                "audit path hash doesn't equal the merkle root"
            );
            assert!(
                merkle_audit_path.contains_block_id(&block_ids[index]),
                "audit path does not contain that block id"
            );
        }
    }
}
