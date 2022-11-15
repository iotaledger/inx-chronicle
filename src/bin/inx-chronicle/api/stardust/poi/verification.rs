// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use chronicle::types::{node::MilestoneKeyRange, tangle::MilestoneIndex};

#[derive(Clone)]
pub struct MilestoneKeyManager {
    key_ranges: Box<[MilestoneKeyRange]>,
}

impl MilestoneKeyManager {
    pub fn new(mut key_ranges: Box<[MilestoneKeyRange]>) -> Self {
        key_ranges.sort();

        Self { key_ranges }
    }

    pub fn get_valid_public_keys_for_index(&self, index: MilestoneIndex) -> Vec<String> {
        let mut public_keys = HashSet::with_capacity(self.key_ranges.len());
        for key_range in self.key_ranges.iter() {
            match (key_range.start, key_range.end) {
                (start, _) if start > index => break,
                (start, end) if index <= end || start == end => {
                    public_keys.insert(key_range.public_key.clone());
                }
                (_, _) => continue,
            }
        }
        public_keys.into_iter().collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_manager_is_sorted() {
        let key_ranges = vec![
            MilestoneKeyRange {
                public_key: "0xa".to_string(),
                start: 42.into(),
                end: 1000.into(),
            },
            MilestoneKeyRange {
                public_key: "0xb".to_string(),
                start: 21.into(),
                end: 1000.into(),
            },
            MilestoneKeyRange {
                public_key: "0xc".to_string(),
                start: 84.into(),
                end: 1000.into(),
            },
            MilestoneKeyRange {
                public_key: "0xd".to_string(),
                start: 0.into(),
                end: 1000.into(),
            },
        ];

        let key_manager = MilestoneKeyManager::new(key_ranges.into_boxed_slice());

        assert_eq!(key_manager.key_ranges[0].public_key, "0xd");
        assert_eq!(key_manager.key_ranges[0].start.0, 0);
        assert_eq!(key_manager.key_ranges[0].end.0, 1000);

        assert_eq!(key_manager.key_ranges[1].public_key, "0xb");
        assert_eq!(key_manager.key_ranges[1].start.0, 21);
        assert_eq!(key_manager.key_ranges[1].end.0, 1000);

        assert_eq!(key_manager.key_ranges[2].public_key, "0xa");
        assert_eq!(key_manager.key_ranges[2].start.0, 42);
        assert_eq!(key_manager.key_ranges[2].end.0, 1000);

        assert_eq!(key_manager.key_ranges[3].public_key, "0xc");
        assert_eq!(key_manager.key_ranges[3].start.0, 84);
        assert_eq!(key_manager.key_ranges[3].end.0, 1000);
    }
}
