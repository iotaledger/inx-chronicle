// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{address::Address, output::AccountId, slot::SlotIndex};
use mongodb::bson::{self, doc};

use super::queries::{AppendQuery, CreatedQuery, ValidatorQuery};
use crate::db::mongodb::collections::outputs::indexer::queries::AddressQuery;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct DelegationOutputsQuery {
    pub address: Option<Address>,
    pub validator: Option<AccountId>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
}

impl From<DelegationOutputsQuery> for bson::Document {
    fn from(query: DelegationOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "output.kind": "delegation" });
        queries.append_query(AddressQuery(query.address));
        queries.append_query(ValidatorQuery(query.validator));
        queries.append_query(CreatedQuery {
            created_before: query.created_before,
            created_after: query.created_after,
        });
        doc! { "$and": queries }
    }
}

#[cfg(test)]
mod test {
    use iota_sdk::types::block::{
        address::Address,
        rand::{address::rand_ed25519_address, output::rand_account_id},
    };
    use mongodb::bson::{self, doc};
    use pretty_assertions::assert_eq;

    use super::DelegationOutputsQuery;
    use crate::model::{address::AddressDto, SerializeToBson};

    #[test]
    fn test_alias_query_everything() {
        let address = Address::from(rand_ed25519_address());
        let validator = rand_account_id();
        let query = DelegationOutputsQuery {
            address: Some(address.clone()),
            validator: Some(validator),
            created_before: Some(10000.into()),
            created_after: Some(1000.into()),
        };
        let address = AddressDto::from(address);
        let query_doc = doc! {
            "$and": [
                { "output.kind": "alias" },
                { "details.address": address.clone() },
                { "details.validator": validator.to_bson() },
                { "metadata.slot_booked": { "$lt": 10000 } },
                { "metadata.slot_booked": { "$gt": 1000 } },
            ]
        };
        assert_eq!(query_doc, bson::Document::from(query));
    }

    #[test]
    fn test_alias_query_all_false() {
        let query = DelegationOutputsQuery {
            created_before: Some(10000.into()),
            ..Default::default()
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "alias" },
                { "metadata.slot_booked": { "$lt": 10000 } }
            ]
        };
        assert_eq!(query_doc, bson::Document::from(query));
    }
}
