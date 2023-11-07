// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{address::Address, slot::SlotIndex};
use mongodb::bson::{self, doc};

use super::queries::{
    AppendQuery, CreatedQuery, GovernorQuery, IssuerQuery, SenderQuery, StateControllerQuery, UnlockableByAddressQuery,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct AnchorOutputsQuery {
    pub state_controller: Option<Address>,
    pub governor: Option<Address>,
    pub issuer: Option<Address>,
    pub sender: Option<Address>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
    pub unlockable_by_address: Option<Address>,
}

impl From<AnchorOutputsQuery> for bson::Document {
    fn from(query: AnchorOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "output.kind": "anchor" });
        queries.append_query(StateControllerQuery(query.state_controller));
        queries.append_query(GovernorQuery(query.governor));
        queries.append_query(IssuerQuery(query.issuer));
        queries.append_query(SenderQuery(query.sender));
        queries.append_query(CreatedQuery {
            created_before: query.created_before,
            created_after: query.created_after,
        });
        queries.append_query(UnlockableByAddressQuery(query.unlockable_by_address));
        doc! { "$and": queries }
    }
}

// #[cfg(test)]
// mod test {
//     use iota_sdk::types::block::{address::Address, rand::address::rand_ed25519_address};
//     use mongodb::bson::{self, doc};
//     use pretty_assertions::assert_eq;

//     use super::AnchorOutputsQuery;
//     use crate::model::address::AddressDto;

//     #[test]
//     fn test_anchor_query_everything() {
//         let address = Address::from(rand_ed25519_address());
//         let query = AnchorOutputsQuery {
//             state_controller: Some(address.clone()),
//             governor: Some(address.clone()),
//             issuer: Some(address.clone()),
//             sender: Some(address.clone()),
//             created_before: Some(10000.into()),
//             created_after: Some(1000.into()),
//             unlockable_by_address: Some(address.clone()),
//         };
//         let address = AddressDto::from(address);
//         let query_doc = doc! {
//             "$and": [
//                 { "output.kind": "anchor" },
//                 { "details.state_controller_address": address.clone() },
//                 { "details.governor_address": address.clone() },
//                 { "details.issuer": address.clone() },
//                 { "details.sender": address },
//                 { "metadata.slot_booked": { "$lt": 10000 } },
//                 { "metadata.slot_booked": { "$gt": 1000 } },
//                 // TODO: unlockable by address
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }

//     #[test]
//     fn test_anchor_query_all_false() {
//         let query = AnchorOutputsQuery {
//             created_before: Some(10000.into()),
//             ..Default::default()
//         };
//         let query_doc = doc! {
//             "$and": [
//                 { "output.kind": "anchor" },
//                 { "metadata.booked.milestone_timestamp": { "$lt": 10000 } }
//             ]
//         };
//         assert_eq!(query_doc, bson::Document::from(query));
//     }
// }
