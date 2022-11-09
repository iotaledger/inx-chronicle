// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crypto::hashes::{Digest, Output};
use iota_types::block::BlockId;

/// Leaf prefix.
const LEAF_HASH_PREFIX: u8 = 0x00;
/// Node sub-tree prefix.
const NODE_HASH_PREFIX: u8 = 0x01;

/// A Merkle tree hasher that is generic over the hash function being used.
pub struct MerkleTreeHasher<D> {
    _phantom: PhantomData<D>,
}

impl<D: Default + Digest> MerkleTreeHasher<D> {
    /// Creates a new Merkle tree hasher.
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }

    /// Returns the Merkle root for a list of [`BlockId`]s.
    pub fn root(&mut self, block_ids: &[BlockId]) -> Vec<u8> {
        self.digest_inner(block_ids).to_vec()
    }

    /// Returns the hash of a Merkle tree leaf.
    fn leaf(&mut self, block_id: BlockId) -> Output<D> {
        let mut hasher = D::default();

        hasher.update([LEAF_HASH_PREFIX]);
        hasher.update(block_id);
        hasher.finalize()
    }

    /// Returns the hash of a Merkle tree node.
    fn node(&mut self, block_ids: &[BlockId]) -> Output<D> {
        let mut hasher = D::default();

        let mid = largest_power_of_two_lte_number((block_ids.len() - 1) as u32);
        let (left, right) = block_ids.split_at(mid);

        hasher.update([NODE_HASH_PREFIX]);
        hasher.update(self.digest_inner(left));
        hasher.update(self.digest_inner(right));
        hasher.finalize()
    }

    fn digest_inner(&mut self, block_ids: &[BlockId]) -> Output<D> {
        match block_ids {
            [] => self.empty(),
            [block_id] => self.leaf(*block_id),
            _ => self.node(block_ids),
        }
    }

    fn empty(&mut self) -> Output<D> {
        D::digest([])
    }
}

fn largest_power_of_two_lte_number(number: u32) -> usize {
    debug_assert!(number > 0);
    1 << (32 - number.leading_zeros() - 1)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crypto::hashes::blake2b::Blake2b256;

    use super::*;

    #[test]
    #[should_panic]
    fn test_largest_power_of_two_lte_number_panic() {
        let _ = largest_power_of_two_lte_number(0);
    }

    #[test]
    fn test_largest_power_of_two_lte_number() {
        assert_eq!(2u32.pow(0) as usize, largest_power_of_two_lte_number(1));
        assert_eq!(2u32.pow(1) as usize, largest_power_of_two_lte_number(2));
        assert_eq!(2u32.pow(1) as usize, largest_power_of_two_lte_number(3));
        assert_eq!(2u32.pow(2) as usize, largest_power_of_two_lte_number(4));
        assert_eq!(2u32.pow(31) as usize, largest_power_of_two_lte_number(u32::MAX));
    }

    #[test]
    fn test_merkle_tree_hasher_empty() {
        let root = MerkleTreeHasher::<Blake2b256>::new().root(&[]);
        assert_eq!(
            prefix_hex::encode(root),
            "0x0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
        )
    }

    #[test]
    fn test_merkle_tree_hasher_single() {
        let root = MerkleTreeHasher::<Blake2b256>::new()
            .root(&[BlockId::from_str("0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c649").unwrap()]);

        assert_eq!(
            prefix_hex::encode(root),
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

        let root = MerkleTreeHasher::<Blake2b256>::new().root(&block_ids);

        assert_eq!(
            prefix_hex::encode(root),
            "0xbf67ce7ba23e8c0951b5abaec4f5524360d2c26d971ff226d3359fa70cdb0beb"
        )
    }
}
