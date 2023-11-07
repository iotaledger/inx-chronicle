// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{address::Address, output::TokenId, slot::SlotIndex};
use mongodb::bson::{self, doc};

use super::queries::{
    AddressQuery, AppendQuery, CreatedQuery, ExpirationQuery, NativeTokensQuery, SenderQuery,
    StorageDepositReturnQuery, TagQuery, TimelockQuery,
};
use crate::{db::mongodb::collections::outputs::indexer::queries::UnlockableByAddressQuery, model::tag::Tag};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct BasicOutputsQuery {
    pub address: Option<Address>,
    pub has_native_tokens: Option<bool>,
    pub native_token: Option<TokenId>,
    pub has_storage_deposit_return: Option<bool>,
    pub storage_deposit_return_address: Option<Address>,
    pub has_timelock: Option<bool>,
    pub timelocked_before: Option<SlotIndex>,
    pub timelocked_after: Option<SlotIndex>,
    pub has_expiration: Option<bool>,
    pub expires_before: Option<SlotIndex>,
    pub expires_after: Option<SlotIndex>,
    pub expiration_return_address: Option<Address>,
    pub sender: Option<Address>,
    pub tag: Option<Tag>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
    pub unlockable_by_address: Option<Address>,
}

impl From<BasicOutputsQuery> for bson::Document {
    fn from(query: BasicOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "output.kind": "basic" });
        queries.append_query(AddressQuery(query.address));
        queries.append_query(NativeTokensQuery {
            has_native_tokens: query.has_native_tokens,
            native_token: query.native_token,
        });
        queries.append_query(StorageDepositReturnQuery {
            has_storage_return_condition: query.has_storage_deposit_return,
            storage_return_address: query.storage_deposit_return_address,
        });
        queries.append_query(TimelockQuery {
            has_timelock_condition: query.has_timelock,
            timelocked_before: query.timelocked_before,
            timelocked_after: query.timelocked_after,
        });
        queries.append_query(ExpirationQuery {
            has_expiration_condition: query.has_expiration,
            expires_before: query.expires_before,
            expires_after: query.expires_after,
            expiration_return_address: query.expiration_return_address,
        });
        queries.append_query(SenderQuery(query.sender));
        queries.append_query(TagQuery(query.tag));
        queries.append_query(CreatedQuery {
            created_before: query.created_before,
            created_after: query.created_after,
        });
        queries.append_query(UnlockableByAddressQuery(query.unlockable_by_address));
        doc! { "$and": queries }
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{self, doc};
//     use pretty_assertions::assert_eq;
//     use primitive_types::U256;

//     use super::BasicOutputsQuery;
//     use crate::model::{
//         payload::transaction::output::Tag,
//         utxo::{Address, NativeTokenAmount},
//     };

//     #[test]
//     fn test_basic_query_everything() {
//         let address = Address::rand_ed25519();
//         let query = BasicOutputsQuery {
//             address: Some(address),
//             has_native_tokens: Some(true),
//             min_native_token_count: Some(100.into()),
//             max_native_token_count: Some(1000.into()),
//             has_storage_deposit_return: Some(true),
//             storage_deposit_return_address: Some(address),
//             has_timelock: Some(true),
//             timelocked_before: Some(10000.into()),
//             timelocked_after: Some(1000.into()),
//             has_expiration: Some(true),
//             expires_before: Some(10000.into()),
//             expires_after: Some(1000.into()),
//             expiration_return_address: Some(address),
//             sender: Some(address),
//             tag: Some(Tag::from("my_tag")),
//             created_before: Some(10000.into()),
//             created_after: Some(1000.into()),
//         };
//         let query_doc = doc! {
//             "$and": [
//                 { "output.kind": "basic" },
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
//                 { "output.storage_deposit_return_unlock_condition": { "$exists": true } },
//                 { "output.storage_deposit_return_unlock_condition.return_address": address },
//                 { "output.timelock_unlock_condition": { "$exists": true } },
//                 { "output.timelock_unlock_condition.timestamp": { "$lt": 10000 } },
//                 { "output.timelock_unlock_condition.timestamp": { "$gt": 1000 } },
//                 { "output.expiration_unlock_condition": { "$exists": true } },
//                 { "output.expiration_unlock_condition.timestamp": { "$lt": 10000 } },
//                 { "output.expiration_unlock_condition.timestamp": { "$gt": 1000 } },
//                 { "output.expiration_unlock_condition.return_address": address },
//                 { "output.features": { "$elemMatch": {
//                     "kind": "sender",
//                     "address": address
//                 } } },
//                 { "output.features": { "$elemMatch": {
//                     "kind": "tag",
//                     "data": Tag::from("my_tag"),
//                 } } },
//                 { "metadata.booked.milestone_timestamp": { "$lt": 10000 } },
//                 { "metadata.booked.milestone_timestamp": { "$gt": 1000 } },
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }

//     #[test]
//     fn test_basic_query_all_false() {
//         let address = Address::rand_ed25519();
//         let query = BasicOutputsQuery {
//             address: Some(address),
//             has_native_tokens: Some(false),
//             min_native_token_count: Some(100.into()),
//             max_native_token_count: Some(1000.into()),
//             has_storage_deposit_return: Some(false),
//             storage_deposit_return_address: Some(address),
//             has_timelock: Some(false),
//             timelocked_before: Some(10000.into()),
//             timelocked_after: Some(1000.into()),
//             has_expiration: Some(false),
//             expires_before: Some(10000.into()),
//             expires_after: Some(1000.into()),
//             expiration_return_address: Some(address),
//             sender: None,
//             tag: Some(Tag::from("my_tag")),
//             created_before: Some(10000.into()),
//             created_after: Some(1000.into()),
//         };
//         let query_doc = doc! {
//             "$and": [
//                 { "output.kind": "basic" },
//                 { "details.address": address },
//                 { "output.native_tokens": { "$eq": [] } },
//                 { "output.storage_deposit_return_unlock_condition": { "$exists": false } },
//                 { "output.storage_deposit_return_unlock_condition.return_address": address },
//                 { "output.timelock_unlock_condition": { "$exists": false } },
//                 { "output.timelock_unlock_condition.timestamp": { "$lt": 10000 } },
//                 { "output.timelock_unlock_condition.timestamp": { "$gt": 1000 } },
//                 { "output.expiration_unlock_condition": { "$exists": false } },
//                 { "output.expiration_unlock_condition.timestamp": { "$lt": 10000 } },
//                 { "output.expiration_unlock_condition.timestamp": { "$gt": 1000 } },
//                 { "output.expiration_unlock_condition.return_address": address },
//                 { "output.features": { "$elemMatch": {
//                     "kind": "tag",
//                     "data": Tag::from("my_tag"),
//                 } } },
//                 { "metadata.booked.milestone_timestamp": { "$lt": 10000 } },
//                 { "metadata.booked.milestone_timestamp": { "$gt": 1000 } },
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }

//     #[test]
//     fn test_basic_query_all_true() {
//         let query = BasicOutputsQuery {
//             has_native_tokens: Some(true),
//             has_storage_deposit_return: Some(true),
//             has_timelock: Some(true),
//             has_expiration: Some(true),
//             ..Default::default()
//         };
//         let query_doc = doc! {
//             "$and": [
//                 { "output.kind": "basic" },
//                 { "output.native_tokens": { "$ne": [] } },
//                 { "output.storage_deposit_return_unlock_condition": { "$exists": true } },
//                 { "output.timelock_unlock_condition": { "$exists": true } },
//                 { "output.expiration_unlock_condition": { "$exists": true } },
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }
// }
