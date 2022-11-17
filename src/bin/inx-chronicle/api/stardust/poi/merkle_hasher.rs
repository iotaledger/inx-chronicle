// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::{Digest, Output};

use super::{
    merkle_proof::{Hashable, MerkleProof},
    PoIError,
};

const LEAF_HASH_PREFIX: u8 = 0;
const NODE_HASH_PREFIX: u8 = 1;

/// A Merkle tree hasher.
pub struct MerkleHasher<H>(PhantomData<H>);

impl<H: Default + Digest> MerkleHasher<H> {
    /// Hash data using the provided hasher type.
    pub fn hash(data: &[impl AsRef<[u8]>]) -> Output<H> {
        match data {
            [] => Self::hash_empty(),
            [leaf] => Self::hash_leaf(leaf),
            _ => {
                let k = largest_power_of_two(data.len());
                let l = Self::hash(&data[..k]);
                let r = Self::hash(&data[k..]);
                Self::hash_node(&l, &r)
            }
        }
    }

    fn hash_empty() -> Output<H> {
        H::digest([])
    }

    /// Hash a terminating leaf of the tree.
    pub fn hash_leaf(l: impl AsRef<[u8]>) -> Output<H> {
        let mut hasher = H::default();
        hasher.update([LEAF_HASH_PREFIX]);
        hasher.update(l);
        hasher.finalize()
    }

    /// Hash a subtree.
    pub fn hash_node(l: impl AsRef<[u8]>, r: impl AsRef<[u8]>) -> Output<H> {
        let mut hasher = H::default();
        hasher.update([NODE_HASH_PREFIX]);
        hasher.update(l);
        hasher.update(r);
        hasher.finalize()
    }

    /// Create a merkle proof given a list of block IDs and a chosen block ID. The chosen leaf will become a
    /// value node, and the path will contain all hashes above it. The remaining branches will be terminated early.
    pub fn create_proof(block_ids: &[BlockId], chosen_block_id: &BlockId) -> Result<MerkleProof<H>, PoIError> {
        let index = block_ids
            .iter()
            .position(|id| id == chosen_block_id)
            .ok_or(PoIError::InvalidRequest("invalid BlockId"))?;
        Self::create_proof_from_index(block_ids, index)
    }

    // NOTE: `block_ids` is the list of past-cone block ids in "White Flag" order.
    fn create_proof_from_index(block_ids: &[BlockId], index: usize) -> Result<MerkleProof<H>, PoIError> {
        let n = block_ids.len();
        if n < 2 {
            Err(PoIError::InvalidInput("cannot create proof for less than 2 block ids"))
        } else if index >= n {
            Err(PoIError::InvalidInput("given index is out of bounds"))
        } else {
            let data = block_ids.iter().map(|block_id| block_id.0).collect::<Vec<_>>();
            Ok(Self::compute_proof(&data, index))
        }
    }

    /// Recursively compute a merkle tree.
    fn compute_proof(data: &[[u8; BlockId::LENGTH]], index: usize) -> MerkleProof<H> {
        let n = data.len();
        debug_assert!(index < n);
        match n {
            0 => unreachable!("zero"),
            1 => unreachable!("one"),
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
                let k = super::merkle_hasher::largest_power_of_two(n);
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

/// Returns the largest power of 2 less than a given number `n`.
///
/// __NOTE__: Panics for `n < 2`.
pub(crate) fn largest_power_of_two(n: usize) -> usize {
    debug_assert!(n > 1);
    1 << (bit_length((n - 1) as u32) - 1)
}

const fn bit_length(n: u32) -> u32 {
    32 - n.leading_zeros()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chronicle::types::stardust::block::BlockId;
    use crypto::hashes::blake2b::Blake2b256;

    use super::*;
    use crate::api::stardust::poi::merkle_proof::MerkleProofDto;

    impl MerkleHasher<Blake2b256> {
        pub fn hash_block_ids(data: &[BlockId]) -> Output<Blake2b256> {
            let data = data.iter().map(|id| &id.0[..]).collect::<Vec<_>>();
            Self::hash(&data[..])
        }
    }

    #[test]
    #[should_panic]
    fn test_largest_power_of_two_panics_for_0() {
        let _ = largest_power_of_two(0);
    }

    #[test]
    #[should_panic]
    fn test_largest_power_of_two_panics_for_1() {
        let _ = largest_power_of_two(1);
    }

    #[test]
    fn test_largest_power_of_two_lte_number() {
        assert_eq!(2u32.pow(0) as usize, largest_power_of_two(2));
        assert_eq!(2u32.pow(1) as usize, largest_power_of_two(3));
        assert_eq!(2u32.pow(1) as usize, largest_power_of_two(4));
        assert_eq!(2u32.pow(31) as usize, largest_power_of_two(u32::MAX as usize));
    }

    #[test]
    fn test_merkle_tree_hasher_empty() {
        let root = MerkleHasher::hash_block_ids(&[]);
        assert_eq!(
            prefix_hex::encode(root.as_slice()),
            "0x0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
        )
    }

    #[test]
    fn test_merkle_tree_hasher_single() {
        let root = MerkleHasher::hash_block_ids(&[BlockId::from_str(
            "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c649",
        )
        .unwrap()]);

        assert_eq!(
            prefix_hex::encode(root.as_slice()),
            "0x3d1399c64ff0ae6a074afa4cd2ce4eab8d5c499c1da6afdd1d84b7447cc00544"
        )
    }

    #[test]
    fn test_merkle_tree_root() {
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

        let merkle_root = MerkleHasher::hash_block_ids(&block_ids);

        assert_eq!(
            prefix_hex::encode(merkle_root.as_slice()),
            "0xbf67ce7ba23e8c0951b5abaec4f5524360d2c26d971ff226d3359fa70cdb0beb"
        )
    }

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
            let proof = MerkleHasher::<Blake2b256>::create_proof_from_index(&block_ids, index).unwrap();
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
