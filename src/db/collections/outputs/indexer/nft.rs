// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::{self, doc},
    error::Error,
};
use primitive_types::U256;

use super::{
    queries::{
        AddressQuery, AppendQuery, CreatedQuery, ExpirationQuery, IssuerQuery, NativeTokensQuery, SenderQuery,
        StorageDepositReturnQuery, TagQuery, TimelockQuery,
    },
    OutputDocument,
};
use crate::{
    db::MongoDb,
    types::{
        stardust::{
            block::{Address, NftId, OutputId},
            milestone::MilestoneTimestamp,
        },
        tangle::MilestoneIndex,
    },
};

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct NftOutputResult {
    pub ledger_index: MilestoneIndex,
    pub output_id: OutputId,
}

/// Implements the queries for the core API.
impl MongoDb {
    /// Gets the current unspent nft output id with the given nft id.
    pub async fn get_nft_output_by_id(&self, nft_id: NftId) -> Result<Option<NftOutputResult>, Error> {
        let ledger_index = self.get_ledger_index().await?;
        if let Some(ledger_index) = ledger_index {
            let res = self
                .0
                .collection::<OutputDocument>(OutputDocument::COLLECTION)
                .find_one(
                    doc! {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "output.nft_id": nft_id,
                        "metadata.spent": null,
                    },
                    None,
                )
                .await?;
            Ok(res.map(|doc| NftOutputResult {
                ledger_index,
                output_id: doc.output_id,
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Debug, Default)]
#[allow(missing_docs)]
pub struct NftOutputsQuery {
    pub address: Option<Address>,
    pub issuer: Option<Address>,
    pub sender: Option<Address>,
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
    pub tag: Option<String>,
    pub created_before: Option<MilestoneTimestamp>,
    pub created_after: Option<MilestoneTimestamp>,
}

impl From<NftOutputsQuery> for bson::Document {
    fn from(query: NftOutputsQuery) -> Self {
        let mut queries = Vec::new();
        queries.push(doc! { "output.kind": "nft" });
        queries.append_query(AddressQuery(query.address));
        queries.append_query(IssuerQuery(query.issuer));
        queries.append_query(SenderQuery(query.sender));
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

    use super::NftOutputsQuery;
    use crate::types::stardust::block::{Address, NativeTokenAmount};

    #[test]
    fn test_nft_query_everything() {
        let address = Address::from(bee::Address::Ed25519(bee_test::rand::address::rand_ed25519_address()));
        let query = NftOutputsQuery {
            address: Some(address),
            issuer: Some(address),
            sender: Some(address),
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
            tag: Some("my_tag".to_string()),
            created_before: Some(10000.into()),
            created_after: Some(1000.into()),
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "nft" },
                { "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "address",
                        "address": address
                    }
                } },
                { "output.features": { "$elemMatch": {
                    "kind": "issuer",
                    "address": address
                } } },
                { "output.features": { "$elemMatch": {
                    "kind": "sender",
                    "address": address
                } } },
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
    fn test_nft_query_all_false() {
        let address = Address::from(bee::Address::Ed25519(bee_test::rand::address::rand_ed25519_address()));
        let query = NftOutputsQuery {
            address: Some(address),
            issuer: None,
            sender: None,
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
            tag: Some("my_tag".to_string()),
            created_before: Some(10000.into()),
            created_after: Some(1000.into()),
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "nft" },
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
    fn test_nft_query_all_true() {
        let query = NftOutputsQuery {
            has_native_tokens: Some(true),
            has_storage_return_condition: Some(true),
            has_timelock_condition: Some(true),
            has_expiration_condition: Some(true),
            ..Default::default()
        };
        let query_doc = doc! {
            "$and": [
                { "output.kind": "nft" },
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
