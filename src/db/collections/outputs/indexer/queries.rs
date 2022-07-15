// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::bson::{self, doc, Document};
use primitive_types::U256;

use crate::types::stardust::{
    block::{Address, TokenAmount},
    milestone::MilestoneTimestamp,
};

/// Defines how a query is appended to a list of `$and` queries.
pub(super) trait AppendToQuery {
    fn append_to(self, queries: &mut Vec<Document>);
}

pub(super) trait AppendQuery<Q> {
    fn append_query(&mut self, query: Q);
}

impl<Q: AppendToQuery> AppendQuery<Q> for Vec<Document> {
    fn append_query(&mut self, query: Q) {
        query.append_to(self)
    }
}

/// Queries for a feature of type `issuer`.
pub(super) struct IssuerQuery(pub(super) Option<Address>);

impl AppendToQuery for IssuerQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "output.features": {
                    "$elemMatch": {
                        "kind": "issuer",
                        "address": address
                    }
                }
            });
        }
    }
}

/// Queries for a feature of type `sender`.
pub(super) struct SenderQuery(pub(super) Option<Address>);

impl AppendToQuery for SenderQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "output.features": {
                    "$elemMatch": {
                        "kind": "sender",
                        "address": address
                    }
                }
            });
        }
    }
}

/// Queries for a feature of type `tag`.
pub(super) struct TagQuery(pub(super) Option<String>);

impl AppendToQuery for TagQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(tag) = self.0 {
            queries.push(doc! {
                "output.features": {
                    "$elemMatch": {
                        "kind": "tag",
                        "data": bson::to_bson(&serde_bytes::Bytes::new(tag.as_bytes())).unwrap()
                    }
                }
            });
        }
    }
}

/// Queries for native tokens.
pub(super) struct NativeTokensQuery {
    pub(super) has_native_tokens: Option<bool>,
    pub(super) min_native_token_count: Option<U256>,
    pub(super) max_native_token_count: Option<U256>,
}

impl AppendToQuery for NativeTokensQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(false) = self.has_native_tokens {
            queries.push(doc! {
                "output.native_tokens": { "$eq": [] }
            });
        } else {
            if matches!(self.has_native_tokens, Some(true))
                || self.min_native_token_count.is_some()
                || self.max_native_token_count.is_some()
            {
                queries.push(doc! {
                    "output.native_tokens": { "$ne": [] }
                });
            }
            if let Some(min_native_token_count) = self.min_native_token_count {
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
            if let Some(max_native_token_count) = self.max_native_token_count {
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
    }
}

/// Queries for an unlock condition of type `address`.
pub(super) struct AddressQuery(pub(super) Option<Address>);

impl AppendToQuery for AddressQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "address",
                        "address": address
                    }
                }
            });
        }
    }
}

/// Queries for an unlock condition of type `state_controller_address`.
pub(super) struct StateControllerQuery(pub(super) Option<Address>);

impl AppendToQuery for StateControllerQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "state_controller_address",
                        "address": address
                    }
                }
            });
        }
    }
}

/// Queries for an unlock condition of type `governor_address`.
pub(super) struct GovernorQuery(pub(super) Option<Address>);

impl AppendToQuery for GovernorQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "governor_address",
                        "address": address
                    }
                }
            });
        }
    }
}

/// Queries for an unlock condition of type `immutable_alias_address`.
pub(super) struct ImmutableAliasAddressQuery(pub(super) Option<Address>);

impl AppendToQuery for ImmutableAliasAddressQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "output.unlock_conditions": {
                    "$elemMatch": {
                        "kind": "immutable_alias_address",
                        "address": address
                    }
                }
            });
        }
    }
}

/// Queries for an unlock condition of type `storage_deposit_return`.
pub(super) struct StorageDepositReturnQuery {
    pub(super) has_storage_return_condition: Option<bool>,
    pub(super) storage_return_address: Option<Address>,
}

impl AppendToQuery for StorageDepositReturnQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        match (self.storage_return_address, self.has_storage_return_condition) {
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
    }
}

/// Queries for an unlock condition of type `timelock`.
pub(super) struct TimelockQuery {
    pub(super) has_timelock_condition: Option<bool>,
    pub(super) timelocked_before: Option<MilestoneTimestamp>,
    pub(super) timelocked_after: Option<MilestoneTimestamp>,
}

impl AppendToQuery for TimelockQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if matches!(self.has_timelock_condition, Some(false)) {
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
            let mut doc = match self.has_timelock_condition {
                Some(true) => Some(doc! { "kind": "timelock" }),
                _ => None,
            };
            if let Some(timelocked_before) = self.timelocked_before {
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
            if let Some(timelocked_after) = self.timelocked_after {
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
    }
}

/// Queries for an unlock condition of type `expiration`.
pub(super) struct ExpirationQuery {
    pub(super) has_expiration_condition: Option<bool>,
    pub(super) expires_before: Option<MilestoneTimestamp>,
    pub(super) expires_after: Option<MilestoneTimestamp>,
    pub(super) expiration_return_address: Option<Address>,
}

impl AppendToQuery for ExpirationQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if matches!(self.has_expiration_condition, Some(false)) {
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
            let mut doc = match self.has_expiration_condition {
                Some(true) => Some(doc! { "kind": "expiration" }),
                _ => None,
            };
            if let Some(expires_before) = self.expires_before {
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
            if let Some(expires_after) = self.expires_after {
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
            if let Some(expiration_return_address) = self.expiration_return_address {
                doc.get_or_insert_with(|| doc! { "kind": "expiration" })
                    .insert("return_address", expiration_return_address);
            }
            if let Some(doc) = doc {
                queries.push(doc! { "output.unlock_conditions": { "$elemMatch": doc } });
            }
        }
    }
}

/// Queries for created (booked) time.
pub(super) struct CreatedQuery {
    pub(super) created_before: Option<MilestoneTimestamp>,
    pub(super) created_after: Option<MilestoneTimestamp>,
}

impl AppendToQuery for CreatedQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        match (self.created_before, self.created_after) {
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
    }
}
