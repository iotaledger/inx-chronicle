// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{address::Address, slot::SlotIndex};
use mongodb::bson::{self, doc};

use super::queries::{AppendQuery, CreatedQuery, IssuerQuery, SenderQuery};
use crate::db::mongodb::collections::outputs::indexer::queries::AddressQuery;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct AccountOutputsQuery {
    pub address: Option<Address>,
    pub issuer: Option<Address>,
    pub sender: Option<Address>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
}

impl From<AccountOutputsQuery> for bson::Document {
    fn from(query: AccountOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "details.kind": "account" });
        queries.append_query(AddressQuery(query.address));
        queries.append_query(IssuerQuery(query.issuer));
        queries.append_query(SenderQuery(query.sender));
        queries.append_query(CreatedQuery {
            created_before: query.created_before,
            created_after: query.created_after,
        });
        doc! { "$and": queries }
    }
}

// #[cfg(test)]
// mod test {
//     use iota_sdk::types::block::{address::Address, rand::address::rand_ed25519_address};
//     use mongodb::bson::{self, doc};
//     use pretty_assertions::assert_eq;

//     use super::AccountOutputsQuery;
//     use crate::model::address::AddressDto;

//     #[test]
//     fn test_alias_query_everything() {
//         let address = Address::from(rand_ed25519_address());
//         let query = AccountOutputsQuery {
//             address: Some(address.clone()),
//             issuer: Some(address.clone()),
//             sender: Some(address.clone()),
//             created_before: Some(10000.into()),
//             created_after: Some(1000.into()),
//         };
//         let address = AddressDto::from(address);
//         let query_doc = doc! {
//             "$and": [
//                 { "details.kind": "account" },
//                 { "details.address": address.clone() },
//                 { "details.issuer": address.clone() },
//                 { "details.sender": address },
//                 { "metadata.slot_booked": { "$lt": 10000 } },
//                 { "metadata.slot_booked": { "$gt": 1000 } },
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }

//     #[test]
//     fn test_alias_query_all_false() {
//         let query = AccountOutputsQuery {
//             created_before: Some(10000.into()),
//             ..Default::default()
//         };
//         let query_doc = doc! {
//             "$and": [
//                 { "details.kind": "account" },
//                 { "metadata.booked.milestone_timestamp": { "$lt": 10000 } }
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }
// }
