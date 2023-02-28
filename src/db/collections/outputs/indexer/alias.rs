// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::bson::{self, doc};
use primitive_types::U256;

use super::queries::{
    AppendQuery, CreatedQuery, GovernorQuery, IssuerQuery, NativeTokensQuery, SenderQuery, StateControllerQuery,
};
use crate::types::stardust::tangle::{block::Address, milestone::MilestoneTimestamp};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct AliasOutputsQuery {
    pub state_controller: Option<Address>,
    pub governor: Option<Address>,
    pub issuer: Option<Address>,
    pub sender: Option<Address>,
    pub has_native_tokens: Option<bool>,
    pub min_native_token_count: Option<U256>,
    pub max_native_token_count: Option<U256>,
    pub created_before: Option<MilestoneTimestamp>,
    pub created_after: Option<MilestoneTimestamp>,
}

impl From<AliasOutputsQuery> for bson::Document {
    fn from(query: AliasOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "output.kind": "alias" });
        queries.append_query(StateControllerQuery(query.state_controller));
        queries.append_query(GovernorQuery(query.governor));
        queries.append_query(IssuerQuery(query.issuer));
        queries.append_query(SenderQuery(query.sender));
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

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{self, doc};
    use primitive_types::U256;

    use super::AliasOutputsQuery;
    use crate::types::stardust::block::{output::NativeTokenAmount, Address};

    #[test]
    fn test_alias_query_everything() {
        let address = Address::rand_ed25519();
        let query = AliasOutputsQuery {
            state_controller: Some(address),
            governor: Some(address),
            issuer: Some(address),
            sender: Some(address),
            has_native_tokens: Some(true),
            min_native_token_count: Some(100.into()),
            max_native_token_count: Some(1000.into()),
            created_before: Some(10000.into()),
            created_after: Some(1000.into()),
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "alias" },
                { "output.state_controller_address_unlock_condition.address": address },
                { "output.governor_address_unlock_condition.address": address },
                { "output.features": {
                    "$elemMatch": {
                        "kind": "issuer",
                        "address": address
                    }
                } },
                { "output.features": {
                    "$elemMatch": {
                        "kind": "sender",
                        "address": address
                    }
                } },
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
    fn test_alias_query_all_false() {
        let query = AliasOutputsQuery {
            has_native_tokens: Some(false),
            min_native_token_count: Some(100.into()),
            max_native_token_count: Some(1000.into()),
            created_before: Some(10000.into()),
            ..Default::default()
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "alias" },
                { "output.native_tokens": { "$eq": [] } },
                { "metadata.booked.milestone_timestamp": { "$lt": 10000 } }
            ]
        };
        assert_eq!(query_doc, bson::Document::from(query));
    }
}
