// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc},
    error::Error,
};
use primitive_types::U256;

use super::{
    queries::{
        AddressQuery, AppendQuery, CreatedQuery, ExpirationQuery, NativeTokensQuery, SenderQuery,
        StorageDepositReturnQuery, TagQuery, TimelockQuery,
    },
    OutputDocument, OutputResult, OutputsResult,
};
use crate::{
    db::MongoDb,
    types::{
        stardust::{
            block::{Address, OutputId},
            milestone::MilestoneTimestamp,
        },
        tangle::MilestoneIndex,
    },
};

/// Implements the queries for the core API.
impl MongoDb {
    /// Gets basic outputs that match the provided query.
    pub async fn get_basic_outputs(
        &self,
        query: BasicOutputsQuery,
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
pub struct BasicOutputsQuery {
    pub address: Option<Address>,
    pub has_native_tokens: Option<bool>,
    pub min_native_token_count: Option<U256>,
    pub max_native_token_count: Option<U256>,
    pub has_storage_return_condition: Option<bool>,
    pub storage_return_address: Option<Address>,
    pub has_timelock_condition: Option<bool>,
    pub timelocked_before: Option<MilestoneTimestamp>,
    pub timelocked_after: Option<MilestoneTimestamp>,
    pub has_expiration_condition: Option<bool>,
    pub expires_before: Option<MilestoneTimestamp>,
    pub expires_after: Option<MilestoneTimestamp>,
    pub expiration_return_address: Option<Address>,
    pub sender: Option<Address>,
    pub tag: Option<String>,
    pub created_before: Option<MilestoneTimestamp>,
    pub created_after: Option<MilestoneTimestamp>,
}

impl From<BasicOutputsQuery> for bson::Document {
    fn from(query: BasicOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "output.kind": "basic" });
        queries.append_query(AddressQuery(query.address));
        queries.append_query(NativeTokensQuery {
            has_native_tokens: query.has_native_tokens,
            min_native_token_count: query.min_native_token_count,
            max_native_token_count: query.max_native_token_count,
        });
        queries.append_query(StorageDepositReturnQuery {
            has_storage_return_condition: query.has_storage_return_condition,
            storage_return_address: query.storage_return_address,
        });
        queries.append_query(TimelockQuery {
            has_timelock_condition: query.has_timelock_condition,
            timelocked_before: query.timelocked_before,
            timelocked_after: query.timelocked_after,
        });
        queries.append_query(ExpirationQuery {
            has_expiration_condition: query.has_expiration_condition,
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
        doc! { "$and": queries }
    }
}

#[cfg(test)]
mod test {
    use bee_block_stardust::address as bee;
    use mongodb::bson::{self, doc};
    use primitive_types::U256;

    use super::BasicOutputsQuery;
    use crate::types::stardust::block::{Address, TokenAmount};

    #[test]
    fn test_basic_query_everything() {
        let address = Address::from(bee::Address::Ed25519(bee_test::rand::address::rand_ed25519_address()));
        let query = BasicOutputsQuery {
            address: Some(address),
            has_native_tokens: Some(true),
            min_native_token_count: Some(100.into()),
            max_native_token_count: Some(1000.into()),
            has_storage_return_condition: Some(true),
            storage_return_address: Some(address),
            has_timelock_condition: Some(true),
            timelocked_before: Some(10000.into()),
            timelocked_after: Some(1000.into()),
            has_expiration_condition: Some(true),
            expires_before: Some(10000.into()),
            expires_after: Some(1000.into()),
            expiration_return_address: Some(address),
            sender: Some(address),
            tag: Some("my_tag".to_string()),
            created_before: Some(10000.into()),
            created_after: Some(1000.into()),
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "basic" },
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "address",
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
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "storage_deposit_return",
                        "return_address": address
                    }
                } },
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "timelock",
                        "timestamp": {
                            "$gt": 1000,
                            "$lt": 10000
                        }
                    }
                } },
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "expiration",
                        "return_address": address,
                        "timestamp": {
                            "$gt": 1000,
                            "$lt": 10000
                        }
                    }
                } },
                { "output.features": { "$elemMatch": {
                    "kind": "sender",
                    "address": address
                } } },
                { "output.features": { "$elemMatch": {
                    "kind": "tag",
                    "data": bson::to_bson(&serde_bytes::Bytes::new("my_tag".as_bytes())).unwrap()
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
    fn test_basic_query_all_false() {
        let address = Address::from(bee::Address::Ed25519(bee_test::rand::address::rand_ed25519_address()));
        let query = BasicOutputsQuery {
            address: Some(address),
            has_native_tokens: Some(false),
            min_native_token_count: Some(100.into()),
            max_native_token_count: Some(1000.into()),
            has_storage_return_condition: Some(false),
            storage_return_address: Some(address),
            has_timelock_condition: Some(false),
            timelocked_before: Some(10000.into()),
            timelocked_after: Some(1000.into()),
            has_expiration_condition: Some(false),
            expires_before: Some(10000.into()),
            expires_after: Some(1000.into()),
            expiration_return_address: Some(address),
            sender: None,
            tag: Some("my_tag".to_string()),
            created_before: Some(10000.into()),
            created_after: Some(1000.into()),
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "basic" },
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "address",
                        "address": address
                    }
                } },
                { "output.native_tokens": { "$eq": [] } },
                { "output.unlock_conditions": {
                    "$not": {
                        "$elemMatch": {
                            "kind": "storage_deposit_return",
                        }
                    }
                } },
                { "output.unlock_conditions": {
                    "$not": {
                        "$elemMatch": {
                            "kind": "timelock",
                        }
                    }
                } },
                { "output.unlock_conditions": {
                    "$not": {
                        "$elemMatch": {
                            "kind": "expiration",
                        }
                    }
                } },
                { "output.features": { "$elemMatch": {
                    "kind": "tag",
                    "data": bson::to_bson(&serde_bytes::Bytes::new("my_tag".as_bytes())).unwrap()
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
    fn test_basic_query_all_true() {
        let query = BasicOutputsQuery {
            has_native_tokens: Some(true),
            has_storage_return_condition: Some(true),
            has_timelock_condition: Some(true),
            has_expiration_condition: Some(true),
            ..Default::default()
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "basic" },
                { "output.native_tokens": { "$ne": [] } },
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "storage_deposit_return",
                    }
                } },
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "timelock",
                    }
                } },
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "expiration",
                    }
                } },
            ]
        };
        assert_eq!(query_doc, bson::Document::from(query));
    }
}
