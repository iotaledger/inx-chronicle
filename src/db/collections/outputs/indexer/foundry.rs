// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::bson::{self, doc};
use primitive_types::U256;

use super::queries::{AppendQuery, CreatedQuery, ImmutableAliasAddressQuery, NativeTokensQuery};
use crate::types::stardust::{block::Address, milestone::MilestoneTimestamp};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct FoundryOutputsQuery {
    pub alias_address: Option<Address>,
    pub has_native_tokens: Option<bool>,
    pub min_native_token_count: Option<U256>,
    pub max_native_token_count: Option<U256>,
    pub created_before: Option<MilestoneTimestamp>,
    pub created_after: Option<MilestoneTimestamp>,
}

impl From<FoundryOutputsQuery> for bson::Document {
    fn from(query: FoundryOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "output.kind": "foundry" });
        queries.append_query(ImmutableAliasAddressQuery(query.alias_address));
        queries.append_query(NativeTokensQuery {
            has_native_tokens: query.has_native_tokens,
            min_native_token_count: query.min_native_token_count,
            max_native_token_count: query.max_native_token_count,
        });
        queries.append_query(CreatedQuery {
            created_before: query.created_before,
            created_after: query.created_after,
        });
        doc! { "$and": queries }
    }
}

#[cfg(test)]
mod test {
    use bee_block_stardust::address as bee;
    use mongodb::bson::{self, doc};
    use primitive_types::U256;

    use super::FoundryOutputsQuery;
    use crate::types::stardust::block::{Address, NativeTokenAmount};

    #[test]
    fn test_foundry_query_everything() {
        let address = Address::from(bee::Address::Ed25519(
            bee_block_stardust::rand::address::rand_ed25519_address(),
        ));
        let query = FoundryOutputsQuery {
            alias_address: Some(address),
            has_native_tokens: Some(true),
            min_native_token_count: Some(100.into()),
            max_native_token_count: Some(1000.into()),
            created_before: Some(10000.into()),
            created_after: Some(1000.into()),
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "foundry" },
                { "output.immutable_alias_address_unlock_condition.address": address },
                { "output.native_tokens": { "$ne": [] } },
                { "output.native_tokens": { "$not": {
                    "$elemMatch": {
                        "amount": { "$lt": bson::to_bson(&NativeTokenAmount::from(&U256::from(100))).unwrap() }
                    }
                } } },
                { "output.native_tokens": { "$not": {
                    "$elemMatch": {
                        "amount": { "$gt": bson::to_bson(&NativeTokenAmount::from(&U256::from(1000))).unwrap() }
                    }
                } } },
                { "metadata.booked.milestone_timestamp": { "$lt": 10000 } },
                { "metadata.booked.milestone_timestamp": { "$gt": 1000 } },
            ]
        };
        assert_eq!(query_doc, bson::Document::from(query));
    }

    #[test]
    fn test_foundry_query_all_false() {
        let query = FoundryOutputsQuery {
            alias_address: None,
            has_native_tokens: Some(false),
            min_native_token_count: Some(100.into()),
            max_native_token_count: Some(1000.into()),
            created_before: Some(10000.into()),
            created_after: Some(1000.into()),
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "foundry" },
                { "output.native_tokens": { "$eq": [] } },
                { "metadata.booked.milestone_timestamp": { "$lt": 10000 } },
                { "metadata.booked.milestone_timestamp": { "$gt": 1000 } },
            ]
        };
        assert_eq!(query_doc, bson::Document::from(query));
    }

    #[test]
    fn test_foundry_query_all_true() {
        let query = FoundryOutputsQuery {
            has_native_tokens: Some(true),
            ..Default::default()
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "foundry" },
                { "output.native_tokens": { "$ne": [] } },
            ]
        };
        assert_eq!(query_doc, bson::Document::from(query));
    }
}
