// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc},
    error::Error,
};
use primitive_types::U256;

use super::{
    queries::{AppendQuery, CreatedQuery, ImmutableAliasAddressQuery, NativeTokensQuery},
    OutputDocument, OutputResult, OutputsResult,
};
use crate::{
    db::MongoDb,
    types::{
        stardust::{
            block::{Address, FoundryId, OutputId},
            milestone::MilestoneTimestamp,
        },
        tangle::MilestoneIndex,
    },
};

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct FoundryOutputResult {
    pub ledger_index: MilestoneIndex,
    pub output_id: OutputId,
}

/// Implements the queries for the core API.
impl MongoDb {
    /// Gets the current unspent foundry output id with the given foundry id.
    pub async fn get_foundry_output_by_id(&self, foundry_id: FoundryId) -> Result<Option<FoundryOutputResult>, Error> {
        let ledger_index = self.get_ledger_index().await?;
        if let Some(ledger_index) = ledger_index {
            let res = self
                .0
                .collection::<OutputDocument>(OutputDocument::COLLECTION)
                .find_one(
                    doc! {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "output.foundry_id": foundry_id,
                        "metadata.spent": null,
                    },
                    None,
                )
                .await?;
            Ok(res.map(|doc| FoundryOutputResult {
                ledger_index,
                output_id: doc.output_id,
            }))
        } else {
            Ok(None)
        }
    }

    /// Gets foundry outputs that match the provided query.
    pub async fn get_foundry_outputs(
        &self,
        query: FoundryOutputsQuery,
        page_size: usize,
        cursor: Option<(MilestoneIndex, OutputId)>,
    ) -> Result<Option<OutputsResult>, Error> {
        let ledger_index = self.get_ledger_index().await?;
        if let Some(ledger_index) = ledger_index {
            let mut query_doc = bson::Document::from(query);
            let mut additional_queries = vec![doc! { "metadata.booked.milestone_index": { "$lte": ledger_index } }];
            if let Some((start_ms, start_output_id)) = cursor {
                additional_queries.push(doc! { "metadata.booked.milestone_index": { "$lte": start_ms } });
                additional_queries.push(doc! { "output_id": { "$lte": start_output_id } });
            }
            query_doc.insert("$and", additional_queries);
            let match_doc = doc! { "$match": query_doc };
            let outputs = self
                .0
                .collection::<OutputResult>(OutputDocument::COLLECTION)
                .aggregate(
                    vec![
                        match_doc,
                        doc! { "$sort": {
                            "metadata.booked.milestone_index": -1,
                            "output_id": -1
                        } },
                        doc! { "$limit": page_size as i64 },
                        doc! { "$replaceWith": {
                            "output_id": "$output_id",
                            "booked_index": "$metadata.booked.milestone_index"
                        } },
                    ],
                    None,
                )
                .await?
                .map_ok(|doc| bson::from_document::<OutputResult>(doc).unwrap())
                .try_collect::<Vec<_>>()
                .await?;
            Ok(Some(OutputsResult { ledger_index, outputs }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Debug, Default)]
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
    use crate::types::stardust::block::{Address, TokenAmount};

    #[test]
    fn test_foundry_query_everything() {
        let address = Address::from(bee::Address::Ed25519(bee_test::rand::address::rand_ed25519_address()));
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
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "immutable_alias_address",
                        "address": address
                    }
                } },
                { "output.native_tokens": { "$ne": [] } },
                { "output.native_tokens": { "$not": {
                    "$elemMatch": {
                        "amount": { "$lt": bson::to_bson(&TokenAmount::from(&U256::from(100))).unwrap() }
                    }
                } } },
                { "output.native_tokens": { "$not": {
                    "$elemMatch": {
                        "amount": { "$gt": bson::to_bson(&TokenAmount::from(&U256::from(1000))).unwrap() }
                    }
                } } },
                { "metadata.booked.milestone_timestamp": {
                    "$gt": 1000,
                    "$lt": 10000
                } }
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
                { "metadata.booked.milestone_timestamp": {
                    "$gt": 1000,
                    "$lt": 10000
                } }
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
