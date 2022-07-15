// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc},
    error::Error,
};
use primitive_types::U256;

use super::{OutputDocument, OutputResult, OutputsResult};
use crate::{
    db::MongoDb,
    types::{
        stardust::{
            block::{Address, OutputId, TokenAmount},
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

        // Address
        if let Some(address) = query.address {
            queries.push(doc! {
                "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "address",
                        "address": address
                    }
                }
            });
        }

        // Native Tokens
        if let Some(false) = query.has_native_tokens {
            queries.push(doc! {
                "output.native_tokens": { "$eq": [] }
            });
        } else {
            if matches!(query.has_native_tokens, Some(true))
                || query.min_native_token_count.is_some()
                || query.max_native_token_count.is_some()
            {
                queries.push(doc! {
                    "output.native_tokens": { "$ne": [] }
                });
            }
            if let Some(min_native_token_count) = query.min_native_token_count {
                queries.push(doc! {
                    "output.native_tokens": {
                        "$not": {
                            "$elemMatch": {
                                "amount": {
                                    "$lt": bson::to_bson(&TokenAmount::from(&min_native_token_count)).unwrap()
                                }
                            }
                        }
                    }
                });
            }
            if let Some(max_native_token_count) = query.max_native_token_count {
                queries.push(doc! {
                    "output.native_tokens": {
                        "$not": {
                            "$elemMatch": {
                                "amount": {
                                    "$gt": bson::to_bson(&TokenAmount::from(&max_native_token_count)).unwrap()
                                }
                            }
                        }
                    }
                });
            }
        }

        // Storage Return
        match (query.storage_return_address, query.has_storage_return_condition) {
            (_, Some(false)) => {
                queries.push(doc! {
                    "output.unlock_conditions": {
                        "$not": {
                            "$elemMatch": {
                                "kind": "storage_deposit_return",
                            }
                        }
                    }
                });
            }
            (Some(storage_return_address), _) => {
                queries.push(doc! {
                    "output.unlock_conditions": {
                        "$elemMatch": {
                            "kind": "storage_deposit_return",
                            "return_address": storage_return_address
                        }
                    }
                });
            }
            (None, Some(true)) => {
                queries.push(doc! {
                    "output.unlock_conditions": {
                        "$elemMatch": {
                            "kind": "storage_deposit_return",
                        }
                    }
                });
            }
            _ => (),
        }

        // Timelock
        if matches!(query.has_timelock_condition, Some(false)) {
            queries.push(doc! {
                "output.unlock_conditions": {
                    "$not": {
                        "$elemMatch": {
                            "kind": "timelock",
                        }
                    }
                }
            });
        } else {
            let mut doc = match query.has_timelock_condition {
                Some(true) => Some(doc! { "kind": "timelock" }),
                _ => None,
            };
            if let Some(timelocked_before) = query.timelocked_before {
                let d = doc.get_or_insert_with(|| doc! { "kind": "timelock" });
                match d.get_document_mut("timestamp").ok() {
                    Some(ts) => {
                        ts.insert("$lt", timelocked_before);
                    }
                    None => {
                        d.insert("timestamp", doc! {"$lt": timelocked_before });
                    }
                }
            }
            if let Some(timelocked_after) = query.timelocked_after {
                let d = doc.get_or_insert_with(|| doc! { "kind": "timelock" });
                match d.get_document_mut("timestamp").ok() {
                    Some(ts) => {
                        ts.insert("$gt", timelocked_after);
                    }
                    None => {
                        d.insert("timestamp", doc! {"$gt": timelocked_after });
                    }
                }
            }
            if let Some(doc) = doc {
                queries.push(doc! { "output.unlock_conditions": { "$elemMatch": doc } });
            }
        }

        // Expiration
        if matches!(query.has_expiration_condition, Some(false)) {
            queries.push(doc! {
                "output.unlock_conditions": {
                    "$not": {
                        "$elemMatch": {
                            "kind": "expiration",
                        }
                    }
                }
            });
        } else {
            let mut doc = match query.has_expiration_condition {
                Some(true) => Some(doc! { "kind": "expiration" }),
                _ => None,
            };
            if let Some(expires_before) = query.expires_before {
                let d = doc.get_or_insert_with(|| doc! { "kind": "expiration" });
                match d.get_document_mut("timestamp").ok() {
                    Some(ts) => {
                        ts.insert("$lt", expires_before);
                    }
                    None => {
                        d.insert("timestamp", doc! {"$lt": expires_before });
                    }
                }
            }
            if let Some(expires_after) = query.expires_after {
                let d = doc.get_or_insert_with(|| doc! { "kind": "expiration" });
                match d.get_document_mut("timestamp").ok() {
                    Some(ts) => {
                        ts.insert("$gt", expires_after);
                    }
                    None => {
                        d.insert("timestamp", doc! {"$gt": expires_after });
                    }
                }
            }
            if let Some(expiration_return_address) = query.expiration_return_address {
                doc.get_or_insert_with(|| doc! { "kind": "expiration" })
                    .insert("return_address", expiration_return_address);
            }
            if let Some(doc) = doc {
                queries.push(doc! { "output.unlock_conditions": { "$elemMatch": doc } });
            }
        }

        // Sender
        if let Some(sender) = query.sender {
            queries.push(doc! {
                "output.features": {
                    "$elemMatch": {
                        "kind": "sender",
                        "address": sender
                    }
                }
            });
        }

        // Tag
        if let Some(tag) = query.tag {
            queries.push(doc! {
                "output.features": {
                    "$elemMatch": {
                        "kind": "tag",
                        "data": bson::to_bson(&serde_bytes::Bytes::new(tag.as_bytes())).unwrap()
                    }
                }
            });
        }

        // Created (booked)
        match (query.created_before, query.created_after) {
            (Some(created_before), Some(created_after)) => {
                queries.push(doc! {
                    "metadata.booked.milestone_timestamp": {
                        "$gt": created_after,
                        "$lt": created_before,
                    }
                });
            }
            (Some(created_before), None) => {
                queries.push(doc! {
                    "metadata.booked.milestone_timestamp": {
                        "$lt": created_before,
                    }
                });
            }
            (None, Some(created_after)) => {
                queries.push(doc! {
                    "metadata.booked.milestone_timestamp": {
                        "$gt": created_after,
                    }
                });
            }
            _ => (),
        }
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
