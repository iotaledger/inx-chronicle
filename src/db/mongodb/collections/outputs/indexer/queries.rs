// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{address::Address, slot::SlotIndex};
use mongodb::bson::{doc, Document};
use primitive_types::U256;

use crate::model::{
    payload::transaction::output::{AddressDto, Tag},
    SerializeToBson,
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
                        "address": AddressDto::from(address)
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
                        "address": AddressDto::from(address)
                    }
                }
            });
        }
    }
}

/// Queries for a feature of type `tag`.
pub(super) struct TagQuery(pub(super) Option<Tag>);

impl AppendToQuery for TagQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(tag) = self.0 {
            queries.push(doc! {
                "output.features": {
                    "$elemMatch": {
                        "kind": "tag",
                        "data": tag,
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
                                    "$lt": min_native_token_count.to_bson()
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
                                    "$gt": max_native_token_count.to_bson()
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
                "details.address": AddressDto::from(address)
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
                "output.governor_address_unlock_condition.address": AddressDto::from(address)
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
        if let Some(has_storage_return_condition) = self.has_storage_return_condition {
            queries.push(doc! {
                "output.storage_deposit_return_unlock_condition": { "$exists": has_storage_return_condition }
            });
        }
        if let Some(storage_return_address) = self.storage_return_address {
            queries.push(doc! {
                "output.storage_deposit_return_unlock_condition.return_address": AddressDto::from(storage_return_address)
            });
        }
    }
}

/// Queries for an unlock condition of type `timelock`.
pub(super) struct TimelockQuery {
    pub(super) has_timelock_condition: Option<bool>,
    pub(super) timelocked_before: Option<SlotIndex>,
    pub(super) timelocked_after: Option<SlotIndex>,
}

impl AppendToQuery for TimelockQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(has_timelock_condition) = self.has_timelock_condition {
            queries.push(doc! {
                "output.timelock_unlock_condition": { "$exists": has_timelock_condition }
            });
        }
        if let Some(timelocked_before) = self.timelocked_before {
            queries.push(doc! {
                "output.timelock_unlock_condition.timestamp": { "$lt": timelocked_before.0 }
            });
        }
        if let Some(timelocked_after) = self.timelocked_after {
            queries.push(doc! {
                "output.timelock_unlock_condition.timestamp": { "$gt": timelocked_after.0 }
            });
        }
    }
}

/// Queries for an unlock condition of type `expiration`.
pub(super) struct ExpirationQuery {
    pub(super) has_expiration_condition: Option<bool>,
    pub(super) expires_before: Option<SlotIndex>,
    pub(super) expires_after: Option<SlotIndex>,
    pub(super) expiration_return_address: Option<Address>,
}

impl AppendToQuery for ExpirationQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(has_expiration_condition) = self.has_expiration_condition {
            queries.push(doc! {
                "output.expiration_unlock_condition": { "$exists": has_expiration_condition }
            });
        }
        if let Some(expires_before) = self.expires_before {
            queries.push(doc! {
                "output.expiration_unlock_condition.timestamp": { "$lt": expires_before.0 }
            });
        }
        if let Some(expires_after) = self.expires_after {
            queries.push(doc! {
                "output.expiration_unlock_condition.timestamp": { "$gt": expires_after.0 }
            });
        }
        if let Some(expiration_return_address) = self.expiration_return_address {
            queries.push(doc! {
                "output.expiration_unlock_condition.return_address": AddressDto::from(expiration_return_address)
            });
        }
    }
}

/// Queries for created (booked) time.
pub(super) struct CreatedQuery {
    pub(super) created_before: Option<SlotIndex>,
    pub(super) created_after: Option<SlotIndex>,
}

impl AppendToQuery for CreatedQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(created_before) = self.created_before {
            queries.push(doc! {
                "metadata.booked.milestone_timestamp": { "$lt": created_before.0 }
            });
        }
        if let Some(created_after) = self.created_after {
            queries.push(doc! {
                "metadata.booked.milestone_timestamp": { "$gt": created_after.0 }
            });
        }
    }
}
