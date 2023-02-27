// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::bson::{self, doc, Document};
use primitive_types::U256;

use crate::model::stardust::{output::NativeTokenAmount, payload::milestone::MilestoneTimestamp, Address};

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
                                    "$lt": bson::to_bson(&NativeTokenAmount::from(&min_native_token_count)).unwrap()
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
                                    "$gt": bson::to_bson(&NativeTokenAmount::from(&max_native_token_count)).unwrap()
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
                "output.address_unlock_condition.address": address
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
                "output.state_controller_address_unlock_condition.address": address
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
                "output.governor_address_unlock_condition.address": address
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
                "output.immutable_alias_address_unlock_condition.address": address
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
                "output.storage_deposit_return_unlock_condition.return_address": storage_return_address
            });
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
        if let Some(has_timelock_condition) = self.has_timelock_condition {
            queries.push(doc! {
                "output.timelock_unlock_condition": { "$exists": has_timelock_condition }
            });
        }
        if let Some(timelocked_before) = self.timelocked_before {
            queries.push(doc! {
                "output.timelock_unlock_condition.timestamp": { "$lt": timelocked_before }
            });
        }
        if let Some(timelocked_after) = self.timelocked_after {
            queries.push(doc! {
                "output.timelock_unlock_condition.timestamp": { "$gt": timelocked_after }
            });
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
        if let Some(has_expiration_condition) = self.has_expiration_condition {
            queries.push(doc! {
                "output.expiration_unlock_condition": { "$exists": has_expiration_condition }
            });
        }
        if let Some(expires_before) = self.expires_before {
            queries.push(doc! {
                "output.expiration_unlock_condition.timestamp": { "$lt": expires_before }
            });
        }
        if let Some(expires_after) = self.expires_after {
            queries.push(doc! {
                "output.expiration_unlock_condition.timestamp": { "$gt": expires_after }
            });
        }
        if let Some(expiration_return_address) = self.expiration_return_address {
            queries.push(doc! {
                "output.expiration_unlock_condition.return_address": expiration_return_address
            });
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
        if let Some(created_before) = self.created_before {
            queries.push(doc! {
                "metadata.booked.milestone_timestamp": { "$lt": created_before }
            });
        }
        if let Some(created_after) = self.created_after {
            queries.push(doc! {
                "metadata.booked.milestone_timestamp": { "$gt": created_after }
            });
        }
    }
}
