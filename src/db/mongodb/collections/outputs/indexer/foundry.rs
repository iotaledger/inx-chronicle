// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{
    output::{AccountId, TokenId},
    slot::SlotIndex,
};
use mongodb::bson::{self, doc};

use super::queries::{AccountAddressQuery, AppendQuery, CreatedQuery, NativeTokensQuery};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct FoundryOutputsQuery {
    pub account: Option<AccountId>,
    pub has_native_tokens: Option<bool>,
    pub native_token: Option<TokenId>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
}

impl From<FoundryOutputsQuery> for bson::Document {
    fn from(query: FoundryOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "details.kind": "foundry" });
        queries.append_query(AccountAddressQuery(query.account));
        queries.append_query(NativeTokensQuery {
            has_native_tokens: query.has_native_tokens,
            native_token: query.native_token,
        });
        queries.append_query(CreatedQuery {
            created_before: query.created_before,
            created_after: query.created_after,
        });
        doc! { "$and": queries }
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{self, doc};
//     use pretty_assertions::assert_eq;
//     use primitive_types::U256;

//     use super::FoundryOutputsQuery;

//     #[test]
//     fn test_foundry_query_everything() {
//         let address = Address::rand_ed25519();
//         let query = FoundryOutputsQuery {
//             alias_address: Some(address),
//             has_native_tokens: Some(true),
//             min_native_token_count: Some(100.into()),
//             max_native_token_count: Some(1000.into()),
//             created_before: Some(10000.into()),
//             created_after: Some(1000.into()),
//         };
//         let query_doc = doc! {
//             "$and": [
//                 { "details.kind": "foundry" },
//                 { "details.address": address },
//                 { "output.native_tokens": { "$ne": [] } },
//                 { "output.native_tokens": { "$not": {
//                     "$elemMatch": {
//                         "amount": { "$lt": bson::to_bson(&NativeTokenAmount::from(&U256::from(100))).unwrap() }
//                     }
//                 } } },
//                 { "output.native_tokens": { "$not": {
//                     "$elemMatch": {
//                         "amount": { "$gt": bson::to_bson(&NativeTokenAmount::from(&U256::from(1000))).unwrap() }
//                     }
//                 } } },
//                 { "metadata.booked.milestone_timestamp": { "$lt": 10000 } },
//                 { "metadata.booked.milestone_timestamp": { "$gt": 1000 } },
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }

//     #[test]
//     fn test_foundry_query_all_false() {
//         let query = FoundryOutputsQuery {
//             alias_address: None,
//             has_native_tokens: Some(false),
//             min_native_token_count: Some(100.into()),
//             max_native_token_count: Some(1000.into()),
//             created_before: Some(10000.into()),
//             created_after: Some(1000.into()),
//         };
//         let query_doc = doc! {
//             "$and": [
//                 { "details.kind": "foundry" },
//                 { "output.native_tokens": { "$eq": [] } },
//                 { "metadata.booked.milestone_timestamp": { "$lt": 10000 } },
//                 { "metadata.booked.milestone_timestamp": { "$gt": 1000 } },
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }

//     #[test]
//     fn test_foundry_query_all_true() {
//         let query = FoundryOutputsQuery {
//             has_native_tokens: Some(true),
//             ..Default::default()
//         };
//         let query_doc = doc! {
//             "$and": [
//                 { "details.kind": "foundry" },
//                 { "output.native_tokens": { "$ne": [] } },
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }
// }
